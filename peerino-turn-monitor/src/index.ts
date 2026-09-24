interface Env {
    CF_ACCOUNT_ID: string;
    TURN_EGRESS_THRESHOLD_GB: string;
    MONITOR_INTERVAL_SECONDS: string;
    HEARTBEAT_TTL_SECONDS: string;
    SHUTDOWN_TTL_SECONDS: string;
    CF_ANALYTICS_TOKEN: string;
    TURN_GUARD: TURN_GUARD_KV;
    TURN_MONITOR: DurableObjectNamespace;
}

interface TURN_GUARD_KV {
    get(key: string): Promise<string | null>;
    put(key: string, value: string, options?: { expirationTtl?: number }): Promise<void>;
    delete(key: string): Promise<void>;
}

declare interface DurableObjectId {}
declare interface DurableObjectNamespace {
    idFromName(name: string): DurableObjectId;
    get(id: DurableObjectId): DurableObjectStub;
}
declare interface DurableObjectStub {
    fetch(request: Request): Promise<Response>;
}
declare interface DurableObjectState {
    storage: {
        get<T>(key: string): Promise<T | undefined>;
        put<T>(key: string, value: T): Promise<void>;
        setAlarm(scheduledTime: number): Promise<void>;
    };
}

const GRAPHQL_URL = 'https://api.cloudflare.com/client/v4/graphql';
const SHUTDOWN_KEY = 'global_shutdown';
const HEARTBEAT_STORAGE_KEY = 'last_run_ms';
const LAST_REFRESH_STORAGE_KEY = 'last_refresh_ms';

function buildQuery(accountTag: string, since: string): string {
    const safeTag = accountTag.replace(/"/g, '');
    const safeSince = since.replace(/"/g, '');
    return `
          query {
            viewer {
              accounts(filter: { accountTag: "${safeTag}" }) {
                callsTurnUsageAdaptiveGroups(
                  limit: 10000
                  filter: { date_geq: "${safeSince}" }
                ) {
                  sum { egressBytes }
                }
              }
            }
          }
        `;
}

function getMonthStartDate(): string {
    const now = new Date();
    const start = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), 1));
    return start.toISOString().slice(0, 10); // "YYYY-MM-DD"
}

async function fetchEgressBytes(env: Env, since: string): Promise<number> {
    const query = buildQuery(env.CF_ACCOUNT_ID, since);

    const response = await fetch(GRAPHQL_URL, {
        method: 'POST',
        headers: {
            Authorization: `Bearer ${env.CF_ANALYTICS_TOKEN}`,
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ query }),
    });

    if (!response.ok) {
        const body = await response.text();
        throw new Error(`GraphQL HTTP ${response.status}: ${body.slice(0, 500)}`);
    }

    const json = await response.json();
    if (json.errors?.length) {
        throw new Error(`GraphQL errors: ${JSON.stringify(json.errors)}`);
    }

    const accounts = json?.data?.viewer?.accounts;
    if (!Array.isArray(accounts) || accounts.length === 0) return 0;

    const groups = accounts[0].callsTurnUsageAdaptiveGroups;
    if (!Array.isArray(groups)) return 0;

    return groups.reduce(
        (sum: number, group: any) => sum + (group.sum?.egressBytes || 0),
        0
    );
}

async function runMonitorCheck(env: Env): Promise<{ egress: number; thresholdBytes: number; shutdown: boolean; changed: boolean }> {
    const egress = await fetchEgressBytes(env, getMonthStartDate());
    const thresholdBytes = Math.floor(parseFloat(env.TURN_EGRESS_THRESHOLD_GB || '250') * 1024 * 1024 * 1024);
    const shutdown = egress > thresholdBytes;
    const currentRaw = await env.TURN_GUARD.get(SHUTDOWN_KEY);
    const changed = shutdown !== (currentRaw === 'true');
    if (changed) {
        if (shutdown) {
            await env.TURN_GUARD.put(SHUTDOWN_KEY, 'true', { expirationTtl: parseInt(env.SHUTDOWN_TTL_SECONDS || '21600', 10) });
        } else {
            await env.TURN_GUARD.delete(SHUTDOWN_KEY);
        }
    }
    return { egress, thresholdBytes, shutdown, changed };
}

export class TurnMonitor {
    state: DurableObjectState;
    env: Env;

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;
    }

    async alarm(): Promise<void> {
        try {
            const result = await runMonitorCheck(this.env);
            await this.state.storage.put(HEARTBEAT_STORAGE_KEY, Date.now());
            if (result.shutdown && !result.changed) {
                const lastRefresh = await this.state.storage.get<number>(LAST_REFRESH_STORAGE_KEY);
                const now = Date.now();
                if (!lastRefresh || now - lastRefresh > 5 * 60 * 60 * 1000) {
                    await this.env.TURN_GUARD.put(SHUTDOWN_KEY, 'true', { expirationTtl: parseInt(this.env.SHUTDOWN_TTL_SECONDS || '21600', 10) });
                    await this.state.storage.put(LAST_REFRESH_STORAGE_KEY, now);
                }
            } else if (result.changed) {
                await this.state.storage.put(LAST_REFRESH_STORAGE_KEY, Date.now());
            }
            console.log(`[turn-monitor] egress=${result.egress} threshold=${result.thresholdBytes} shutdown=${result.shutdown} changed=${result.changed}`);
        } catch (e) {
            console.error('[turn-monitor] alarm error:', e);
        } finally {
            const interval = parseInt(this.env.MONITOR_INTERVAL_SECONDS || '60', 10);
            await this.state.storage.setAlarm(Date.now() + interval * 1000);
        }
    }

    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);
        if (url.pathname === '/status') {
            const lastRun = await this.state.storage.get<number>(HEARTBEAT_STORAGE_KEY) || 0;
            const ageSeconds = lastRun ? Math.floor((Date.now() - lastRun) / 1000) : -1;
            return new Response(JSON.stringify({ monitor_last_run_ms: lastRun, monitor_age_seconds: ageSeconds, monitor_healthy: ageSeconds >= 0 && ageSeconds < parseInt(this.env.HEARTBEAT_TTL_SECONDS || '900', 10) }), { headers: { 'content-type': 'application/json' } });
        }
        if (url.pathname === '/bootstrap') {
            const interval = parseInt(this.env.MONITOR_INTERVAL_SECONDS || '60', 10);
            const next = Date.now() + interval * 1000;
            await this.state.storage.setAlarm(next);

            // Primo check immediato
            const result = await runMonitorCheck(this.env);
            await this.state.storage.put(HEARTBEAT_STORAGE_KEY, Date.now());

            return new Response(JSON.stringify({ scheduled_at: next, ...result }), {
                headers: { 'content-type': 'application/json' },
            });
        }
        if (url.pathname === '/check') {
            const result = await runMonitorCheck(this.env);
            await this.state.storage.put(HEARTBEAT_STORAGE_KEY, Date.now());
            return new Response(JSON.stringify(result), { headers: { 'content-type': 'application/json' } });
        }
        return new Response('not found', { status: 404 });
    }
}

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);
        const stub = env.TURN_MONITOR.get(env.TURN_MONITOR.idFromName('global'));
        if (url.pathname === '/status') {
            const doJson = await (await stub.fetch(new Request('https://internal/status'))).json();
            const shutdown = (await env.TURN_GUARD.get(SHUTDOWN_KEY)) === 'true';
            return new Response(JSON.stringify({ ...doJson, global_shutdown: shutdown }), { headers: { 'content-type': 'application/json' } });
        }
        if (url.pathname === '/bootstrap') {
            const id = env.TURN_MONITOR.idFromName('global');
            const bootstrapStub = env.TURN_MONITOR.get(id);
            const doResp = await bootstrapStub.fetch(new Request('https://internal/bootstrap'));
            const body = await doResp.text();
            return new Response(JSON.stringify({ bootstrapped: true, result: body }), { headers: { 'content-type': 'application/json' } });
        }
        return new Response('not found', { status: 404 });
    },
};
