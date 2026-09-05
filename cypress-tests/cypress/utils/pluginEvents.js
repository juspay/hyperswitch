/**
 * Let more than one plugin listen to the same Cypress lifecycle event.
 *
 * Cypress registers exactly one handler per event name: in
 * `plugins/child/run_plugins.js` only `task` merges, every other event does
 * `registeredEventsByName[event] = eventId`, so a second registration silently
 * replaces the first. That bites as soon as two things want the same hook —
 * `cypress-mochawesome-reporter` owns `before:run`/`after:run` to merge its
 * report, and this config owns `after:spec` to drop videos for passing specs.
 *
 * Wrapping `on` fans one Cypress registration out to every handler, in
 * registration order, awaiting each.
 *
 * Only the run/spec lifecycle events are multiplexed. The rest — notably
 * `file:preprocessor` and `before:browser:launch`, whose return value Cypress
 * consumes — pass straight through, since fanning those out would discard all
 * but one return value.
 */

const MULTIPLEXED_EVENTS = new Set([
  "before:run",
  "after:run",
  "before:spec",
  "after:spec",
]);

export function multiplexLifecycleEvents(on) {
  const handlers = new Map();

  return function register(event, handler) {
    if (!MULTIPLEXED_EVENTS.has(event)) {
      return on(event, handler);
    }

    const existing = handlers.get(event);
    if (existing) {
      existing.push(handler);
      return undefined;
    }

    const chain = [handler];
    handlers.set(event, chain);

    return on(event, async (...args) => {
      for (const fn of chain) {
        await fn(...args);
      }
    });
  };
}
