CREATE TABLE objects_history (
 id TEXT PRIMARY KEY,
 kind TEXT NOT NULL CHECK(kind IN ('workspace','document','document_link','entity','membership','participation','asset','ownership','valuation','company_policy','exchange_rate','profile_period')),
 created_at TEXT NOT NULL
) STRICT;
INSERT INTO objects_history SELECT id,kind,created_at FROM objects;
DROP TABLE objects;
ALTER TABLE objects_history RENAME TO objects;
CREATE TABLE exchange_rates (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 base_currency TEXT NOT NULL REFERENCES currencies(code),
 quote_currency TEXT NOT NULL REFERENCES currencies(code),
 rate TEXT NOT NULL, effective_on TEXT NOT NULL, source TEXT, notes TEXT NOT NULL,
 CHECK(base_currency<>quote_currency)
) STRICT;
CREATE INDEX exchange_rates_pair_date ON exchange_rates(base_currency,quote_currency,effective_on);
CREATE TABLE profile_periods (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 target_kind TEXT NOT NULL CHECK(target_kind IN('entity','asset')),
 target_id TEXT NOT NULL REFERENCES objects(id),
 profile_revision_id TEXT NOT NULL REFERENCES revisions(id),
 valid_from TEXT NOT NULL, valid_to TEXT,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX profile_periods_target ON profile_periods(target_kind,target_id,valid_from,valid_to);
CREATE INDEX profile_periods_revision ON profile_periods(profile_revision_id);
