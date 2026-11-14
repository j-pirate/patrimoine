CREATE TABLE objects_property (
 id TEXT PRIMARY KEY,
 kind TEXT NOT NULL CHECK(kind IN ('workspace','document','document_link','entity','membership','participation','asset','ownership','valuation','company_policy','exchange_rate','profile_period','property','rental_unit','property_event','lease','lease_terms','rent_due','rent_payment','rent_allocation','rental_expense')),
 created_at TEXT NOT NULL
) STRICT;
INSERT INTO objects_property SELECT id,kind,created_at FROM objects;
DROP TABLE objects;
ALTER TABLE objects_property RENAME TO objects;
CREATE TABLE property_details (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 asset_id TEXT NOT NULL REFERENCES assets(id),
 property_type TEXT NOT NULL, city TEXT NOT NULL, address TEXT NOT NULL,
 surface_m2 TEXT, status TEXT NOT NULL CHECK(status IN('owner_occupied','rented','vacant','renovation','for_sale')),
 valid_from TEXT NOT NULL, valid_to TEXT, notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX property_details_asset ON property_details(asset_id,valid_from,valid_to);
CREATE TABLE rental_units (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 property_asset_id TEXT NOT NULL REFERENCES assets(id), reference TEXT NOT NULL, name TEXT NOT NULL,
 surface_m2 TEXT, valid_from TEXT NOT NULL, valid_to TEXT,
 available_for_rent INTEGER NOT NULL CHECK(available_for_rent IN(0,1)), notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX rental_units_asset ON rental_units(property_asset_id,reference,valid_from,valid_to);
CREATE TABLE property_events (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 asset_id TEXT NOT NULL REFERENCES assets(id),
 kind TEXT NOT NULL CHECK(kind IN('acquisition','opening_tracking','disposal')),
 effective_on TEXT NOT NULL, price TEXT, fees TEXT, currency TEXT NOT NULL REFERENCES currencies(code), notes TEXT NOT NULL
) STRICT;
CREATE INDEX property_events_asset ON property_events(asset_id,effective_on);
CREATE TABLE leases (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 unit_id TEXT NOT NULL REFERENCES objects(id),
 tenant_entity_id TEXT NOT NULL REFERENCES entities(id),
 valid_from TEXT NOT NULL, valid_to TEXT, notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX leases_unit_period ON leases(unit_id,valid_from,valid_to);
CREATE TABLE lease_terms (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 lease_id TEXT NOT NULL REFERENCES objects(id),
 rent TEXT NOT NULL, charges TEXT NOT NULL,
 currency TEXT NOT NULL REFERENCES currencies(code),
 frequency TEXT NOT NULL CHECK(frequency IN ('monthly','quarterly','yearly')),
 due_day INTEGER NOT NULL CHECK(due_day BETWEEN 1 AND 31),
 deposit_required TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT, notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX lease_terms_lease_period ON lease_terms(lease_id,valid_from,valid_to);
CREATE TABLE rent_due_keys (
 due_id TEXT PRIMARY KEY REFERENCES objects(id),
 generation_key TEXT NOT NULL UNIQUE
) STRICT;
CREATE TABLE rent_dues (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 lease_id TEXT NOT NULL REFERENCES objects(id),
 terms_revision_id TEXT NOT NULL REFERENCES lease_terms(revision_id),
 period_start TEXT NOT NULL, period_end TEXT NOT NULL, due_on TEXT NOT NULL,
 rent TEXT NOT NULL, charges TEXT NOT NULL,
 currency TEXT NOT NULL REFERENCES currencies(code), notes TEXT NOT NULL,
 CHECK(period_end>period_start)
) STRICT;
CREATE INDEX rent_dues_lease_period ON rent_dues(lease_id,period_start,period_end,due_on);
CREATE TABLE rent_payments (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 lease_id TEXT NOT NULL REFERENCES objects(id),
 kind TEXT NOT NULL CHECK(kind IN ('receipt','refund')),
 amount TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code), effective_on TEXT NOT NULL,
 original_payment_id TEXT REFERENCES objects(id),
 recipient_entity_id TEXT NOT NULL REFERENCES entities(id),
 reference TEXT NOT NULL, unlinked_reason TEXT NOT NULL
) STRICT;
CREATE INDEX rent_payments_lease_date ON rent_payments(lease_id,effective_on);
CREATE TABLE rent_allocations (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 payment_id TEXT NOT NULL REFERENCES objects(id),
 due_id TEXT NOT NULL REFERENCES objects(id),
 component TEXT NOT NULL CHECK(component IN ('rent','charges')),
 amount TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code), effective_on TEXT NOT NULL
) STRICT;
CREATE INDEX rent_allocations_payment ON rent_allocations(payment_id,effective_on);
CREATE INDEX rent_allocations_due ON rent_allocations(due_id,effective_on);
CREATE TABLE rental_expenses (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 asset_id TEXT NOT NULL REFERENCES assets(id),
 kind TEXT NOT NULL CHECK(kind IN ('operating','tax','other','capital_works','interest','principal','financing_fee')),
 amount TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code),
 effective_on TEXT NOT NULL, paid_on TEXT,
 recoverable INTEGER NOT NULL CHECK(recoverable IN (0,1)), notes TEXT NOT NULL
) STRICT;
CREATE INDEX rental_expenses_asset_date ON rental_expenses(asset_id,effective_on);
