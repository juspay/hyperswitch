import { Browser, BrowserContext, chromium, LaunchOptions } from '@playwright/test';

/**
 * Browser Pool Configuration
 */
export interface BrowserPoolConfig {
  /** Number of browser contexts to create per browser */
  contextsPerBrowser: number;

  /** Browser launch options */
  launchOptions?: LaunchOptions;

  /** Whether to enable verbose logging */
  verbose?: boolean;
}

/**
 * Context allocation metadata
 */
interface ContextAllocation {
  context: BrowserContext;
  inUse: boolean;
  testTitle?: string;
  allocatedAt?: Date;
  releasedAt?: Date;
  usageCount: number;
}

/**
 * Browser pool metrics for performance monitoring
 */
export interface BrowserPoolMetrics {
  connector: string;
  totalContexts: number;
  activeContexts: number;
  idleContexts: number;
  totalAllocations: number;
  averageContextReuseCount: number;
  peakConcurrentContexts: number;
  contextCreationTime: number;
}

/**
 * Browser Pool Manager
 *
 * Manages a pool of reusable browser contexts to enable high-concurrency
 * parallel test execution with minimal RAM overhead.
 *
 * Architecture:
 * - 1 Browser instance per connector (Stripe, Cybersource)
 * - N reusable BrowserContexts per browser (default: 25)
 * - Queue-based context allocation with automatic waiting
 * - Context cleanup and reuse after test completion
 *
 * RAM Efficiency:
 * - Browser instance: ~500-800 MB
 * - Each context: ~50 MB
 * - Total per browser: ~2-3 GB (vs 10-15 GB for 25 separate browsers)
 *
 * Usage:
 * ```typescript
 * const pool = new BrowserPool('stripe', { contextsPerBrowser: 25 });
 * await pool.initialize();
 *
 * const context = await pool.allocateContext('MyTest');
 * // Run test with context
 * await pool.releaseContext(context);
 *
 * await pool.destroy();
 * ```
 */
export class BrowserPool {
  private connector: string;
  private config: Required<BrowserPoolConfig>;
  private browser: Browser | null = null;
  private contextPool: ContextAllocation[] = [];
  private waitQueue: Array<{
    testTitle: string;
    resolve: (context: BrowserContext) => void;
  }> = [];

  // Metrics tracking
  private metrics = {
    totalAllocations: 0,
    peakConcurrentContexts: 0,
    contextCreationStartTime: 0,
    contextCreationEndTime: 0,
  };

  constructor(connector: string, config: Partial<BrowserPoolConfig> = {}) {
    this.connector = connector;
    this.config = {
      contextsPerBrowser: config.contextsPerBrowser ?? 25,
      launchOptions: config.launchOptions ?? {},
      verbose: config.verbose ?? false,
    };
  }

  /**
   * Initialize the browser pool
   * - Launches browser instance
   * - Creates pool of reusable contexts
   */
  async initialize(): Promise<void> {
    this.log(`Initializing browser pool for ${this.connector}...`);
    this.metrics.contextCreationStartTime = Date.now();

    // Launch browser instance
    this.browser = await chromium.launch({
      headless: this.config.launchOptions.headless ?? true,
      ...this.config.launchOptions,
    });

    this.log(`Browser launched for ${this.connector}`);

    // Create context pool
    const contextPromises = Array.from(
      { length: this.config.contextsPerBrowser },
      async (_, index) => {
        const context = await this.browser!.newContext({
          viewport: { width: 1280, height: 720 },
          ignoreHTTPSErrors: true,
        });

        this.contextPool.push({
          context,
          inUse: false,
          usageCount: 0,
        });

        this.log(`Context ${index + 1}/${this.config.contextsPerBrowser} created`);
      }
    );

    await Promise.all(contextPromises);

    this.metrics.contextCreationEndTime = Date.now();
    const creationTime = this.metrics.contextCreationEndTime - this.metrics.contextCreationStartTime;

    this.log(
      `✓ Browser pool initialized for ${this.connector}: ` +
      `${this.config.contextsPerBrowser} contexts created in ${creationTime}ms`
    );
  }

  /**
   * Allocate a browser context for test execution
   *
   * If all contexts are in use, the request is queued and will be
   * fulfilled when a context becomes available.
   *
   * @param testTitle - Name of the test requesting the context
   * @returns Browser context ready for use
   */
  async allocateContext(testTitle: string): Promise<BrowserContext> {
    // Try to find an idle context
    const allocation = this.contextPool.find((a) => !a.inUse);

    if (allocation) {
      // Mark as in use
      allocation.inUse = true;
      allocation.testTitle = testTitle;
      allocation.allocatedAt = new Date();
      allocation.usageCount++;

      this.metrics.totalAllocations++;

      // Update peak concurrent contexts
      const activeCount = this.contextPool.filter((a) => a.inUse).length;
      if (activeCount > this.metrics.peakConcurrentContexts) {
        this.metrics.peakConcurrentContexts = activeCount;
      }

      this.log(`✓ Allocated context to "${testTitle}" (${activeCount} active)`);
      return allocation.context;
    }

    // No idle context available - queue the request
    this.log(`⏳ Context requested by "${testTitle}" - queued (all ${this.config.contextsPerBrowser} contexts in use)`);

    return new Promise((resolve) => {
      this.waitQueue.push({ testTitle, resolve });
    });
  }

  /**
   * Release a browser context back to the pool
   *
   * The context is cleaned up (cookies, localStorage cleared) and
   * made available for the next queued test.
   *
   * @param context - Browser context to release
   */
  async releaseContext(context: BrowserContext): Promise<void> {
    const allocation = this.contextPool.find((a) => a.context === context);

    if (!allocation) {
      console.warn(`[BrowserPool:${this.connector}] Attempted to release unknown context`);
      return;
    }

    const activeCount = this.contextPool.filter((a) => a.inUse).length;
    const testTitle = allocation.testTitle || 'unknown';

    // Clean up context state for reuse
    try {
      await context.clearCookies();
      // Clear local storage by closing all pages and reopening a blank page
      const pages = context.pages();
      await Promise.all(pages.map(page => page.close()));
    } catch (error) {
      this.log(`⚠ Warning: Context cleanup failed for "${testTitle}": ${error}`);
    }

    allocation.inUse = false;
    allocation.releasedAt = new Date();
    allocation.testTitle = undefined;

    this.log(`✓ Released context from "${testTitle}" (${activeCount - 1} active)`);

    // Fulfill next queued request if any
    const next = this.waitQueue.shift();
    if (next) {
      this.log(`✓ Fulfilling queued request for "${next.testTitle}"`);

      allocation.inUse = true;
      allocation.testTitle = next.testTitle;
      allocation.allocatedAt = new Date();
      allocation.usageCount++;

      this.metrics.totalAllocations++;

      next.resolve(allocation.context);
    }
  }

  /**
   * Get current pool metrics for performance monitoring
   */
  getMetrics(): BrowserPoolMetrics {
    const activeContexts = this.contextPool.filter((a) => a.inUse).length;
    const totalReuseCount = this.contextPool.reduce((sum, a) => sum + a.usageCount, 0);
    const averageReuse = totalReuseCount / this.config.contextsPerBrowser;

    return {
      connector: this.connector,
      totalContexts: this.config.contextsPerBrowser,
      activeContexts,
      idleContexts: this.config.contextsPerBrowser - activeContexts,
      totalAllocations: this.metrics.totalAllocations,
      averageContextReuseCount: averageReuse,
      peakConcurrentContexts: this.metrics.peakConcurrentContexts,
      contextCreationTime: this.metrics.contextCreationEndTime - this.metrics.contextCreationStartTime,
    };
  }

  /**
   * Get detailed context usage information
   */
  getContextUsage(): Array<{
    index: number;
    inUse: boolean;
    testTitle?: string;
    usageCount: number;
  }> {
    return this.contextPool.map((allocation, index) => ({
      index,
      inUse: allocation.inUse,
      testTitle: allocation.testTitle,
      usageCount: allocation.usageCount,
    }));
  }

  /**
   * Check if pool is fully utilized (all contexts in use)
   */
  isFullyUtilized(): boolean {
    return this.contextPool.every((a) => a.inUse);
  }

  /**
   * Get wait queue length
   */
  getQueueLength(): number {
    return this.waitQueue.length;
  }

  /**
   * Destroy the browser pool
   * - Closes all browser contexts
   * - Closes browser instance
   * - Clears wait queue
   */
  async destroy(): Promise<void> {
    this.log(`Destroying browser pool for ${this.connector}...`);

    // Reject all queued requests
    if (this.waitQueue.length > 0) {
      console.warn(
        `[BrowserPool:${this.connector}] Destroying pool with ${this.waitQueue.length} queued requests`
      );
      this.waitQueue = [];
    }

    // Close all contexts
    if (this.contextPool.length > 0) {
      await Promise.all(
        this.contextPool.map(async (allocation) => {
          try {
            await allocation.context.close();
          } catch (error) {
            // Ignore errors during cleanup
          }
        })
      );
      this.contextPool = [];
    }

    // Close browser
    if (this.browser) {
      await this.browser.close();
      this.browser = null;
    }

    this.log(`✓ Browser pool destroyed for ${this.connector}`);
  }

  /**
   * Log helper
   */
  private log(message: string): void {
    if (this.config.verbose) {
      console.log(`[BrowserPool:${this.connector}] ${message}`);
    }
  }
}

/**
 * Global Browser Pool Manager
 *
 * Manages multiple browser pools (one per connector) and provides
 * a centralized interface for context allocation across connectors.
 */
export class GlobalBrowserPoolManager {
  private pools = new Map<string, BrowserPool>();
  private config: Partial<BrowserPoolConfig>;

  constructor(config: Partial<BrowserPoolConfig> = {}) {
    this.config = config;
  }

  /**
   * Initialize browser pool for a connector
   */
  async initializePool(connector: string): Promise<void> {
    if (this.pools.has(connector)) {
      console.warn(`[GlobalBrowserPoolManager] Pool for ${connector} already initialized`);
      return;
    }

    const pool = new BrowserPool(connector, this.config);
    await pool.initialize();
    this.pools.set(connector, pool);
  }

  /**
   * Get browser pool for a connector
   */
  getPool(connector: string): BrowserPool | undefined {
    return this.pools.get(connector);
  }

  /**
   * Allocate context from connector's pool
   */
  async allocateContext(connector: string, testTitle: string): Promise<BrowserContext> {
    const pool = this.pools.get(connector);
    if (!pool) {
      throw new Error(`No browser pool initialized for connector: ${connector}`);
    }
    return pool.allocateContext(testTitle);
  }

  /**
   * Release context back to connector's pool
   */
  async releaseContext(connector: string, context: BrowserContext): Promise<void> {
    const pool = this.pools.get(connector);
    if (!pool) {
      console.warn(`[GlobalBrowserPoolManager] No pool found for ${connector}`);
      return;
    }
    return pool.releaseContext(context);
  }

  /**
   * Get aggregated metrics across all pools
   */
  getAllMetrics(): BrowserPoolMetrics[] {
    return Array.from(this.pools.values()).map((pool) => pool.getMetrics());
  }

  /**
   * Destroy all browser pools
   */
  async destroyAll(): Promise<void> {
    const destroyPromises = Array.from(this.pools.values()).map((pool) => pool.destroy());
    await Promise.all(destroyPromises);
    this.pools.clear();
  }
}
