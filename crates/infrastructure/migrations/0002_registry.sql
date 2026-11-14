CREATE TABLE objects_registry (
 id TEXT PRIMARY KEY,
 kind TEXT NOT NULL CHECK(kind IN ('workspace','document','document_link','entity','membership','participation','asset','ownership','valuation','company_policy')),
 created_at TEXT NOT NULL
) STRICT;
INSERT INTO objects_registry SELECT id,kind,created_at FROM objects;
DROP TABLE objects;
ALTER TABLE objects_registry RENAME TO objects;
CREATE TABLE entities (id TEXT PRIMARY KEY REFERENCES objects(id), kind TEXT NOT NULL CHECK(kind IN('person','company'))) STRICT;
CREATE TABLE entity_profiles (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), entity_id TEXT NOT NULL REFERENCES entities(id),
 display_name TEXT NOT NULL, first_name TEXT, last_name TEXT, birth_date TEXT,
 residence_country TEXT REFERENCES countries(code), preferred_currency TEXT NOT NULL REFERENCES currencies(code),
 activity TEXT NOT NULL, notes TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN(0,1))
) STRICT;
CREATE INDEX entity_profiles_entity ON entity_profiles(entity_id);
CREATE TABLE family_memberships (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), person_id TEXT NOT NULL REFERENCES entities(id),
 valid_from TEXT NOT NULL, valid_to TEXT, relationship TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX memberships_person ON family_memberships(person_id,valid_from,valid_to);
CREATE TABLE company_participations (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), owner_entity_id TEXT NOT NULL REFERENCES entities(id),
 company_id TEXT NOT NULL REFERENCES entities(id), legal_share_num TEXT, legal_share_den TEXT,
 economic_share_num TEXT, economic_share_den TEXT, voting_share_num TEXT, voting_share_den TEXT,
 valid_from TEXT NOT NULL, valid_to TEXT,
 CHECK(owner_entity_id<>company_id), CHECK(valid_to IS NULL OR valid_to>valid_from),
 CHECK((legal_share_num IS NULL)=(legal_share_den IS NULL)),
 CHECK((economic_share_num IS NULL)=(economic_share_den IS NULL)),
 CHECK((voting_share_num IS NULL)=(voting_share_den IS NULL))
) STRICT;
CREATE INDEX participations_company ON company_participations(company_id,valid_from,valid_to);
CREATE INDEX participations_owner ON company_participations(owner_entity_id,valid_from,valid_to);
CREATE TABLE assets (id TEXT PRIMARY KEY REFERENCES objects(id)) STRICT;
CREATE TABLE asset_profiles (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), asset_id TEXT NOT NULL REFERENCES assets(id),
 name TEXT NOT NULL, country TEXT NOT NULL REFERENCES countries(code), category TEXT NOT NULL,
 notes TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN(0,1))
) STRICT;
CREATE INDEX asset_profiles_asset ON asset_profiles(asset_id);
CREATE TABLE asset_ownerships (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), asset_id TEXT NOT NULL REFERENCES assets(id),
 owner_entity_id TEXT NOT NULL REFERENCES entities(id), legal_share_num TEXT, legal_share_den TEXT,
 economic_share_num TEXT, economic_share_den TEXT, valid_from TEXT NOT NULL, valid_to TEXT,
 CHECK(valid_to IS NULL OR valid_to>valid_from),
 CHECK((legal_share_num IS NULL)=(legal_share_den IS NULL)),
 CHECK((economic_share_num IS NULL)=(economic_share_den IS NULL))
) STRICT;
CREATE INDEX ownerships_asset ON asset_ownerships(asset_id,valid_from,valid_to);
CREATE INDEX ownerships_owner ON asset_ownerships(owner_entity_id,valid_from,valid_to);
CREATE TABLE valuations (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), target_kind TEXT NOT NULL CHECK(target_kind IN('asset','company')),
 target_id TEXT NOT NULL REFERENCES objects(id), amount TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code),
 effective_on TEXT NOT NULL, source TEXT NOT NULL
) STRICT;
CREATE INDEX valuations_target ON valuations(target_id,effective_on);
CREATE TABLE company_policies (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), company_id TEXT NOT NULL REFERENCES entities(id),
 mode TEXT NOT NULL CHECK(mode IN('detailed','global_equity')), coverage_complete INTEGER NOT NULL CHECK(coverage_complete IN(0,1)),
 liability_qualified INTEGER NOT NULL CHECK(liability_qualified IN(0,1)), valid_from TEXT NOT NULL, valid_to TEXT,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX company_policies_company ON company_policies(company_id,valid_from,valid_to);
