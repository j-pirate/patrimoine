CREATE TABLE objects_metals_companies (
 id TEXT PRIMARY KEY,
 kind TEXT NOT NULL CHECK(kind IN ('workspace','document','document_link','entity','membership','participation','asset','ownership','valuation','company_policy','exchange_rate','profile_period','property','rental_unit','property_event','lease','lease_terms','rent_due','rent_payment','rent_allocation','rental_expense','treasury_account','treasury_transaction','account_reconciliation','cash_link','deposit_movement','debt_contract','debt_terms','debt_movement','debt_receivable_adjustment','debt_schedule','debt_payment_allocation','debt_asset_link','debt_guarantee','debt_coverage','finance_coverage','rent_claim','rent_claim_adjustment','metal_lot','metal_movement','metal_quote','metal_premium','metal_cash_link','company_financial_period','company_distribution')),
 created_at TEXT NOT NULL
) STRICT;
INSERT INTO objects_metals_companies SELECT id,kind,created_at FROM objects;
DROP TABLE objects;
ALTER TABLE objects_metals_companies RENAME TO objects;

CREATE TABLE metal_lots (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), asset_id TEXT NOT NULL REFERENCES assets(id),
 metal TEXT NOT NULL, form TEXT NOT NULL, designation TEXT NOT NULL,
 unit_weight TEXT NOT NULL, weight_unit TEXT NOT NULL, purity TEXT NOT NULL,
 tracking_start TEXT NOT NULL, cost_currency TEXT NOT NULL REFERENCES currencies(code),
 legal_owner_entity_id TEXT NOT NULL REFERENCES entities(id), country TEXT NOT NULL REFERENCES countries(code),
 storage_location TEXT NOT NULL, valuation_basis TEXT NOT NULL, notes TEXT NOT NULL,
 archived INTEGER NOT NULL CHECK(archived IN(0,1))
) STRICT;
CREATE INDEX metal_lots_asset ON metal_lots(asset_id);

CREATE TABLE metal_movements (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), lot_id TEXT NOT NULL REFERENCES objects(id),
 kind TEXT NOT NULL, effective_on TEXT NOT NULL, quantity_change TEXT NOT NULL,
 cost_amount TEXT, trade_amount TEXT, trade_fees TEXT NOT NULL,
 trade_currency TEXT REFERENCES currencies(code), cash_entity_id TEXT NOT NULL,
 counterparty_entity_id TEXT REFERENCES entities(id), country TEXT NOT NULL,
 storage_location TEXT NOT NULL, unlinked_reason TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX metal_movements_lot_date ON metal_movements(lot_id,effective_on);

CREATE TABLE metal_quotes (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), metal TEXT NOT NULL,
 effective_on TEXT NOT NULL, price TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code),
 weight_unit TEXT NOT NULL, source TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX metal_quotes_metal_date ON metal_quotes(metal,effective_on);

CREATE TABLE metal_premiums (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), lot_id TEXT NOT NULL REFERENCES objects(id),
 effective_on TEXT NOT NULL, kind TEXT NOT NULL, value TEXT NOT NULL, currency TEXT NOT NULL,
 discount_per_unit TEXT NOT NULL, source TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX metal_premiums_lot_date ON metal_premiums(lot_id,effective_on);

CREATE TABLE metal_cash_links (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), movement_id TEXT NOT NULL REFERENCES objects(id),
 movement_revision_id TEXT NOT NULL REFERENCES metal_movements(revision_id),
 transaction_revision_id TEXT NOT NULL REFERENCES treasury_transactions(revision_id),
 line_id TEXT NOT NULL, amount TEXT NOT NULL, notes TEXT NOT NULL,
 FOREIGN KEY(transaction_revision_id,line_id) REFERENCES treasury_entries(transaction_revision_id,line_id)
) STRICT;
CREATE INDEX metal_cash_links_movement ON metal_cash_links(movement_id,movement_revision_id);
CREATE INDEX metal_cash_links_entry ON metal_cash_links(transaction_revision_id,line_id);

CREATE TABLE company_financial_periods (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), company_id TEXT NOT NULL REFERENCES entities(id),
 period_start TEXT NOT NULL, period_end TEXT NOT NULL, frequency TEXT NOT NULL,
 currency TEXT NOT NULL REFERENCES currencies(code), revenue TEXT, operating_expenses TEXT,
 operating_result TEXT, net_result TEXT, indicative_cash TEXT, indicative_debt TEXT,
 important_receivables TEXT, dividends_declared TEXT, indicative_global_value TEXT, notes TEXT NOT NULL,
 CHECK(period_end>period_start)
) STRICT;
CREATE INDEX company_financial_periods_company_period ON company_financial_periods(company_id,period_start,period_end);

CREATE TABLE company_distributions (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), company_id TEXT NOT NULL REFERENCES entities(id),
 beneficiary_entity_id TEXT NOT NULL REFERENCES entities(id), kind TEXT NOT NULL, effective_on TEXT NOT NULL,
 amount TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code),
 related_declaration_id TEXT REFERENCES objects(id), unlinked_reason TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX company_distributions_company_date ON company_distributions(company_id,effective_on);
CREATE INDEX company_distributions_declaration ON company_distributions(related_declaration_id);

CREATE TABLE company_distribution_cash_links (
 distribution_revision_id TEXT NOT NULL REFERENCES company_distributions(revision_id), side TEXT NOT NULL,
 transaction_revision_id TEXT NOT NULL REFERENCES treasury_transactions(revision_id), line_id TEXT NOT NULL,
 amount TEXT NOT NULL,
 PRIMARY KEY(distribution_revision_id,side,transaction_revision_id,line_id),
 FOREIGN KEY(transaction_revision_id,line_id) REFERENCES treasury_entries(transaction_revision_id,line_id)
) STRICT;
CREATE INDEX company_distribution_cash_links_entry ON company_distribution_cash_links(transaction_revision_id,line_id);
