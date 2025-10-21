import { BrowserContext } from '@playwright/test';
import { BrowserPool, GlobalBrowserPoolManager } from './BrowserPool';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Test execution phase
 */
export enum ExecutionPhase {
  SETUP = 'setup',
  PARALLEL = 'parallel',
}

/**
 * Test definition
 */
export interface TestDefinition {
  /** Test file path */
  filePath: string;

  /** Test title/name */
  title: string;

  /** Connector (stripe, cybersource) */
  connector: string;

  /** Execution phase (setup or parallel) */
  phase: ExecutionPhase;

  /** Order within phase (for setup tests) */
  order?: number;
}

/**
 * Test execution result
 */
export interface TestExecutionResult {
  test: TestDefinition;
  status: 'passed' | 'failed' | 'skipped';
  duration: number;
  error?: Error;
  startTime: Date;
  endTime: Date;
}

/**
 * Orchestrator metrics
 */
export interface OrchestratorMetrics {
  totalTests: number;
  setupTests: number;
  parallelTests: number;
  testsPerConnector: Record<string, number>;
  totalDuration: number;
  setupPhaseDuration: number;
  parallelPhaseDuration: number;
  averageTestDuration: number;
  peakConcurrentTests: number;
  contextReuseEfficiency: number;
}

/**
 * Parallel Test Orchestrator
 *
 * Custom test orchestrator that implements the multi-tab parallel execution strategy:
 * 1. Phase 1 (Sequential): Run first 4 setup tests per connector sequentially
 * 2. Phase 2 (Parallel): Run remaining tests in parallel using browser context pool
 *
 * Architecture:
 * - 2 browsers (Stripe + Cybersource)
 * - 25 contexts per browser (50 total concurrent tests)
 * - Queue-based test scheduling
 * - Automatic batching for optimal throughput
 *
 * Target Performance:
 * - Setup phase: ~20 seconds (4 tests × 2 connectors, sequential)
 * - Parallel phase: ~60-70 seconds (408 tests × 2 connectors, 50 concurrent)
 * - Total: ~1.5 minutes (vs 20+ minutes sequential)
 * - RAM usage: ~14-15 GB (optimal for 16 GB CI runners)
 */
export class ParallelTestOrchestrator {
  private poolManager: GlobalBrowserPoolManager;
  private tests: TestDefinition[] = [];
  private results: TestExecutionResult[] = [];
  private activeTests = new Map<string, TestDefinition>();

  // Metrics
  private metrics = {
    startTime: 0,
    endTime: 0,
    setupPhaseStartTime: 0,
    setupPhaseEndTime: 0,
    parallelPhaseStartTime: 0,
    parallelPhaseEndTime: 0,
    peakConcurrentTests: 0,
  };

  constructor(
    private config: {
      /** Number of contexts per browser (default: 25) */
      contextsPerBrowser?: number;

      /** Connectors to test (default: ['stripe', 'cybersource']) */
      connectors?: string[];

      /** Base directory for tests */
      testBaseDir?: string;

      /** Enable verbose logging */
      verbose?: boolean;
    } = {}
  ) {
    this.poolManager = new GlobalBrowserPoolManager({
      contextsPerBrowser: config.contextsPerBrowser ?? 25,
      verbose: config.verbose ?? false,
    });
  }

  /**
   * Initialize the orchestrator
   * - Discovers test files
   * - Categorizes into setup vs parallel phases
   * - Initializes browser pools for each connector
   */
  async initialize(): Promise<void> {
    console.log('\n🚀 Initializing Parallel Test Orchestrator...\n');

    const connectors = this.config.connectors ?? ['stripe', 'cybersource'];
    const testBaseDir = this.config.testBaseDir ?? path.join(__dirname, '../tests/e2e');

    // Discover tests
    await this.discoverTests(testBaseDir, connectors);

    console.log(`\n📊 Test Discovery Summary:`);
    console.log(`   Total tests: ${this.tests.length}`);
    console.log(`   Setup tests: ${this.tests.filter(t => t.phase === ExecutionPhase.SETUP).length}`);
    console.log(`   Parallel tests: ${this.tests.filter(t => t.phase === ExecutionPhase.PARALLEL).length}`);

    connectors.forEach(connector => {
      const connectorTests = this.tests.filter(t => t.connector === connector);
      console.log(`   ${connector}: ${connectorTests.length} tests`);
    });

    // Initialize browser pools
    console.log('\n🌐 Initializing browser pools...\n');
    for (const connector of connectors) {
      await this.poolManager.initializePool(connector);
    }

    console.log('✓ Orchestrator initialized\n');
  }

  /**
   * Discover test files and categorize them
   */
  private async discoverTests(baseDir: string, connectors: string[]): Promise<void> {
    const setupDir = path.join(baseDir, 'setup');
    const specDir = path.join(baseDir, 'spec');

    for (const connector of connectors) {
      // Setup tests (00000-00003) - must run sequentially
      const setupTests = [
        { file: '00000-CoreFlows.spec.ts', title: 'Core Flows Setup', order: 0 },
        { file: '00001-AccountCreate.spec.ts', title: 'Account Create', order: 1 },
        { file: '00002-CustomerCreate.spec.ts', title: 'Customer Create', order: 2 },
        { file: '00003-ConnectorCreate.spec.ts', title: 'Connector Create', order: 3 },
      ];

      setupTests.forEach(test => {
        const filePath = path.join(setupDir, test.file);
        if (fs.existsSync(filePath)) {
          this.tests.push({
            filePath,
            title: `[${connector}] ${test.title}`,
            connector,
            phase: ExecutionPhase.SETUP,
            order: test.order,
          });
        }
      });

      // Parallel tests (00004+) - can run concurrently
      if (fs.existsSync(specDir)) {
        const specFiles = fs.readdirSync(specDir)
          .filter(file => file.match(/^\d{5}-.*\.spec\.ts$/))
          .filter(file => {
            const num = parseInt(file.substring(0, 5));
            return num >= 4; // Only tests 00004 and above
          })
          .sort();

        specFiles.forEach(file => {
          const filePath = path.join(specDir, file);
          const testName = file.replace(/^\d{5}-/, '').replace('.spec.ts', '');

          this.tests.push({
            filePath,
            title: `[${connector}] ${testName}`,
            connector,
            phase: ExecutionPhase.PARALLEL,
          });
        });
      }
    }
  }

  /**
   * Execute all tests with the multi-phase strategy
   */
  async execute(): Promise<TestExecutionResult[]> {
    console.log('\n▶️  Starting Test Execution\n');
    this.metrics.startTime = Date.now();

    // Phase 1: Sequential setup tests
    await this.executeSetupPhase();

    // Phase 2: Parallel tests
    await this.executeParallelPhase();

    this.metrics.endTime = Date.now();

    // Print summary
    this.printExecutionSummary();

    return this.results;
  }

  /**
   * Execute setup phase (tests 00000-00003) sequentially per connector
   */
  private async executeSetupPhase(): Promise<void> {
    console.log('📋 Phase 1: Setup Tests (Sequential)\n');
    this.metrics.setupPhaseStartTime = Date.now();

    const setupTests = this.tests
      .filter(t => t.phase === ExecutionPhase.SETUP)
      .sort((a, b) => {
        // First by connector, then by order
        if (a.connector !== b.connector) {
          return a.connector.localeCompare(b.connector);
        }
        return (a.order ?? 0) - (b.order ?? 0);
      });

    // Group by connector
    const testsByConnector = new Map<string, TestDefinition[]>();
    setupTests.forEach(test => {
      if (!testsByConnector.has(test.connector)) {
        testsByConnector.set(test.connector, []);
      }
      testsByConnector.get(test.connector)!.push(test);
    });

    // Execute setup tests per connector in parallel (but sequential within each connector)
    const connectorPromises = Array.from(testsByConnector.entries()).map(
      async ([connector, tests]) => {
        console.log(`   🔧 ${connector} setup started`);

        for (const test of tests) {
          await this.executeTest(test, connector);
        }

        console.log(`   ✓ ${connector} setup completed\n`);
      }
    );

    await Promise.all(connectorPromises);

    this.metrics.setupPhaseEndTime = Date.now();
    const duration = (this.metrics.setupPhaseEndTime - this.metrics.setupPhaseStartTime) / 1000;
    console.log(`✓ Setup phase completed in ${duration.toFixed(1)}s\n`);
  }

  /**
   * Execute parallel phase (tests 00004+) with maximum concurrency
   */
  private async executeParallelPhase(): Promise<void> {
    console.log('⚡ Phase 2: Parallel Tests (50 concurrent)\n');
    this.metrics.parallelPhaseStartTime = Date.now();

    const parallelTests = this.tests.filter(t => t.phase === ExecutionPhase.PARALLEL);

    if (parallelTests.length === 0) {
      console.log('   No parallel tests to execute\n');
      return;
    }

    // Group by connector for tracking
    const testsByConnector = new Map<string, TestDefinition[]>();
    parallelTests.forEach(test => {
      if (!testsByConnector.has(test.connector)) {
        testsByConnector.set(test.connector, []);
      }
      testsByConnector.get(test.connector)!.push(test);
    });

    console.log(`   Total tests: ${parallelTests.length}`);
    testsByConnector.forEach((tests, connector) => {
      console.log(`   ${connector}: ${tests.length} tests`);
    });
    console.log();

    // Execute all tests concurrently (context pool will handle queueing)
    const testPromises = parallelTests.map(test =>
      this.executeTest(test, test.connector)
    );

    // Start progress monitoring
    const progressInterval = this.startProgressMonitoring(parallelTests.length);

    await Promise.all(testPromises);

    clearInterval(progressInterval);

    this.metrics.parallelPhaseEndTime = Date.now();
    const duration = (this.metrics.parallelPhaseEndTime - this.metrics.parallelPhaseStartTime) / 1000;
    console.log(`\n✓ Parallel phase completed in ${duration.toFixed(1)}s\n`);
  }

  /**
   * Execute a single test
   */
  private async executeTest(test: TestDefinition, connector: string): Promise<void> {
    const pool = this.poolManager.getPool(connector);
    if (!pool) {
      throw new Error(`No browser pool for connector: ${connector}`);
    }

    // Allocate context from pool
    const context = await pool.allocateContext(test.title);

    const startTime = new Date();
    const testId = `${connector}:${test.title}`;
    this.activeTests.set(testId, test);

    // Track peak concurrent tests
    if (this.activeTests.size > this.metrics.peakConcurrentTests) {
      this.metrics.peakConcurrentTests = this.activeTests.size;
    }

    let result: TestExecutionResult;

    try {
      // Execute the test
      // In actual implementation, this would invoke the test file
      // For now, we'll simulate with a delay
      await this.runTestFile(test, context);

      const endTime = new Date();
      result = {
        test,
        status: 'passed',
        duration: endTime.getTime() - startTime.getTime(),
        startTime,
        endTime,
      };
    } catch (error) {
      const endTime = new Date();
      result = {
        test,
        status: 'failed',
        duration: endTime.getTime() - startTime.getTime(),
        error: error as Error,
        startTime,
        endTime,
      };
    } finally {
      this.activeTests.delete(testId);
      this.results.push(result);

      // Release context back to pool
      await pool.releaseContext(context);
    }
  }

  /**
   * Run a test file with the provided context
   * This is a placeholder - actual implementation would use Playwright test runner
   */
  private async runTestFile(test: TestDefinition, context: BrowserContext): Promise<void> {
    // Placeholder: simulate test execution
    // In real implementation, this would:
    // 1. Import the test file
    // 2. Inject the context into the test
    // 3. Run the test
    // 4. Capture results

    // For now, simulate with random duration
    const duration = Math.random() * 3000 + 1000; // 1-4 seconds
    await new Promise(resolve => setTimeout(resolve, duration));
  }

  /**
   * Start progress monitoring during parallel phase
   */
  private startProgressMonitoring(totalTests: number): NodeJS.Timeout {
    let lastCompleted = 0;

    return setInterval(() => {
      const completed = this.results.filter(r =>
        r.test.phase === ExecutionPhase.PARALLEL
      ).length;

      if (completed > lastCompleted) {
        const percentage = ((completed / totalTests) * 100).toFixed(1);
        const active = this.activeTests.size;

        console.log(
          `   Progress: ${completed}/${totalTests} (${percentage}%) | ` +
          `Active: ${active} | ` +
          `Passed: ${this.results.filter(r => r.status === 'passed').length}`
        );

        lastCompleted = completed;
      }
    }, 2000); // Update every 2 seconds
  }

  /**
   * Print execution summary
   */
  private printExecutionSummary(): void {
    const totalDuration = (this.metrics.endTime - this.metrics.startTime) / 1000;
    const setupDuration = (this.metrics.setupPhaseEndTime - this.metrics.setupPhaseStartTime) / 1000;
    const parallelDuration = (this.metrics.parallelPhaseEndTime - this.metrics.parallelPhaseStartTime) / 1000;

    const passed = this.results.filter(r => r.status === 'passed').length;
    const failed = this.results.filter(r => r.status === 'failed').length;

    console.log('\n' + '='.repeat(60));
    console.log('📊 EXECUTION SUMMARY');
    console.log('='.repeat(60) + '\n');

    console.log('⏱️  Duration:');
    console.log(`   Total: ${totalDuration.toFixed(1)}s`);
    console.log(`   Setup phase: ${setupDuration.toFixed(1)}s`);
    console.log(`   Parallel phase: ${parallelDuration.toFixed(1)}s\n`);

    console.log('✅ Results:');
    console.log(`   Passed: ${passed}`);
    console.log(`   Failed: ${failed}`);
    console.log(`   Total: ${this.results.length}\n`);

    console.log('⚡ Parallelism:');
    console.log(`   Peak concurrent tests: ${this.metrics.peakConcurrentTests}`);

    const poolMetrics = this.poolManager.getAllMetrics();
    poolMetrics.forEach(metrics => {
      console.log(`   ${metrics.connector}:`);
      console.log(`     - Contexts: ${metrics.totalContexts}`);
      console.log(`     - Total allocations: ${metrics.totalAllocations}`);
      console.log(`     - Avg reuse: ${metrics.averageContextReuseCount.toFixed(1)}x`);
      console.log(`     - Peak concurrent: ${metrics.peakConcurrentContexts}`);
    });

    console.log('\n' + '='.repeat(60) + '\n');
  }

  /**
   * Get execution metrics
   */
  getMetrics(): OrchestratorMetrics {
    const totalDuration = this.metrics.endTime - this.metrics.startTime;
    const setupDuration = this.metrics.setupPhaseEndTime - this.metrics.setupPhaseStartTime;
    const parallelDuration = this.metrics.parallelPhaseEndTime - this.metrics.parallelPhaseStartTime;

    const testsPerConnector: Record<string, number> = {};
    this.tests.forEach(test => {
      testsPerConnector[test.connector] = (testsPerConnector[test.connector] || 0) + 1;
    });

    const totalTestDuration = this.results.reduce((sum, r) => sum + r.duration, 0);
    const averageDuration = totalTestDuration / this.results.length;

    const poolMetrics = this.poolManager.getAllMetrics();
    const totalAllocations = poolMetrics.reduce((sum, m) => sum + m.totalAllocations, 0);
    const totalContexts = poolMetrics.reduce((sum, m) => sum + m.totalContexts, 0);
    const efficiency = totalAllocations / totalContexts;

    return {
      totalTests: this.tests.length,
      setupTests: this.tests.filter(t => t.phase === ExecutionPhase.SETUP).length,
      parallelTests: this.tests.filter(t => t.phase === ExecutionPhase.PARALLEL).length,
      testsPerConnector,
      totalDuration,
      setupPhaseDuration: setupDuration,
      parallelPhaseDuration: parallelDuration,
      averageTestDuration: averageDuration,
      peakConcurrentTests: this.metrics.peakConcurrentTests,
      contextReuseEfficiency: efficiency,
    };
  }

  /**
   * Cleanup and destroy browser pools
   */
  async destroy(): Promise<void> {
    console.log('\n🧹 Cleaning up...\n');
    await this.poolManager.destroyAll();
    console.log('✓ Cleanup complete\n');
  }
}
