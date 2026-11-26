import { describeExecution } from '../src/payments/execution';

const att = (attemptId: string | null, connector: string, status: string) => ({ attemptId, connector, status, errorCode: null });

describe('describeExecution (requested routing vs what Hyperswitch did)', () => {
  it('first requested connector handled the payment: nothing to flag except the unordered attempts', () => {
    const e = describeExecution(['B', 'A'], { connector: 'B', status: 'succeeded' }, [att('p_1', 'B', 'charged')]);
    expect(e.requestedOrder).toEqual(['B', 'A']);
    expect(e.finalConnector).toBe('B');
    expect(e.finalIsFirstRequested).toBe(true);
    expect(e.usedOutsideRequested).toEqual([]);
    expect(e.attemptOrderEstablished).toBe(false);
    expect(e.notes).toHaveLength(1);
  });

  it('fallback inside the requested list: final connector differs from the first requested one', () => {
    const e = describeExecution(['B', 'A'], { connector: 'A', status: 'succeeded' }, [att('p_2', 'A', 'charged'), att('p_1', 'B', 'failure')]);
    expect(e.finalIsFirstRequested).toBe(false);
    expect(e.usedOutsideRequested).toEqual([]);
    // listed alphabetically by connector, never by time
    expect(e.attempts!.map((a) => a.connector)).toEqual(['A', 'B']);
  });

  it('retry continued past the requested list: flagged, and no ordering is claimed', () => {
    const e = describeExecution(['F', 'P'], { connector: 'S', status: 'succeeded' }, [att('p_3', 'S', 'charged'), att('p_1', 'F', 'failure'), att('p_2', 'P', 'failure')]);
    expect(e.usedOutsideRequested).toEqual(['S']);
    expect(e.finalIsFirstRequested).toBe(false);
    expect(e.notes.join(' ')).toMatch(/outside the requested list: S/);
    expect(e.attemptOrderEstablished).toBe(false);
  });

  it('the requested connector was silently replaced (never attempted)', () => {
    const e = describeExecution(['F'], { connector: 'S', status: 'succeeded' }, [att('p_1', 'S', 'charged')]);
    expect(e.usedOutsideRequested).toEqual(['S']);
    expect(e.notes.join(' ')).toMatch(/first requested connector does not appear in the attempts/);
  });

  it('no override sent: nothing is judged against a request', () => {
    const e = describeExecution(null, { connector: 'S', status: 'succeeded' }, [att('p_1', 'S', 'charged')]);
    expect(e.requestedOrder).toBeNull();
    expect(e.finalIsFirstRequested).toBeNull();
    expect(e.usedOutsideRequested).toEqual([]);
    expect(e.notes.join(' ')).toMatch(/no routing override was sent/);
  });

  it('attempts unreadable or response without a connector: unknowns stay null, not guessed', () => {
    const e = describeExecution(['B'], {}, null);
    expect(e.attempts).toBeNull();
    expect(e.finalConnector).toBeNull();
    expect(e.finalIsFirstRequested).toBeNull();
    expect(e.notes.join(' ')).toMatch(/attempts could not be read/);
  });
});
