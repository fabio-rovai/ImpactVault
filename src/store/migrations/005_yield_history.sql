CREATE TABLE IF NOT EXISTS yield_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    adapter_name TEXT NOT NULL,
    apy REAL NOT NULL,
    tvl INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_yield_snapshots_adapter ON yield_snapshots(adapter_name);
CREATE INDEX IF NOT EXISTS idx_yield_snapshots_time ON yield_snapshots(recorded_at);

CREATE TABLE IF NOT EXISTS governance_proposals (
    id INTEGER PRIMARY KEY,
    proposer TEXT NOT NULL,
    call_data TEXT NOT NULL,
    proposed_at TEXT NOT NULL DEFAULT (datetime('now')),
    approvals INTEGER NOT NULL DEFAULT 1,
    executed INTEGER NOT NULL DEFAULT 0,
    executed_at TEXT
);
