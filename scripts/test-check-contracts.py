#!/usr/bin/env python3
"""Source mutation fixtures prove the contract guard detects independent drift.

Fixtures copy only source/configuration into /tmp, and never open user data.
The same real syn/TypeScript extractors used by CI run on every mutation.
"""
from contextlib import contextmanager
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("contracts", ROOT / "scripts/check-contracts.py")
contracts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contracts)


class ContractFixtures(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = contracts.build_extractor()
        cls.temporary = tempfile.TemporaryDirectory(prefix="patrimoine-contract-fixture-")
        cls.root = Path(cls.temporary.name)
        for directory in ("crates/domain/src", "src/services", "src-tauri/src"):
            shutil.copytree(ROOT / directory, cls.root / directory)
        shutil.copy2(ROOT / "tsconfig.app.json", cls.root / "tsconfig.app.json")
        (cls.root / "node_modules").symlink_to(ROOT / "node_modules", target_is_directory=True)

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    @contextmanager
    def mutation(self, path, old, new):
        path = self.root / path
        original = path.read_text()
        self.assertIn(old, original, f"Fixture anchor missing in {path.name}")
        path.write_text(original.replace(old, new, 1))
        try:
            yield
        finally:
            path.write_text(original)

    def check(self):
        return contracts.compare(*contracts.extract(self.root, self.binary))

    def rejects(self, path, old, new, diagnostic):
        with self.mutation(path, old, new):
            try:
                issues, _ = self.check()
                message = "\n".join(issues)
            except contracts.ContractError as error:
                message = str(error)
            self.assertIn(diagnostic, message)

    def test_current_contracts_match(self):
        issues, stats = self.check()
        self.assertEqual([], issues)
        self.assertEqual(stats["invoke_wrappers"], stats["rust_handlers"])
        self.assertEqual(stats["rust_handlers"], stats["registered_commands"])
        self.assertGreater(stats["shared_fields"], 1000)

    def test_scalar_type_drift(self):
        self.rejects("src/services/api.ts", "schema_version: number;", "schema_version: string;", "VaultStatus.schema_version")

    def test_nullability_drift(self):
        self.rejects("src/services/api.ts", "recovery_message: string | null;", "recovery_message: string;", "VaultStatus.recovery_message")

    def test_optional_is_not_nullable(self):
        self.rejects("src/services/api.ts", "recovery_message: string | null;", "recovery_message?: string | null;", "required/optional differs")

    def test_undefined_is_not_a_serialized_field_value(self):
        self.rejects("src/services/api.ts", "workspace_name: string;", "workspace_name: string | undefined;", "Settings.workspace_name")

    def test_missing_required_field(self):
        self.rejects("src/services/api.ts", "  country_count: number;\n", "", "field keys differ")

    def test_list_shape_drift(self):
        self.rejects("src/services/registry-types.ts", "entities: Entity[];", "entities: Entity;", "RegistrySnapshot.entities")

    def test_array_element_drift(self):
        self.rejects("src/services/registry-types.ts", "entities: Entity[];", "entities: Asset[];", "RegistrySnapshot.entities[]")

    def test_inline_nested_object_drift(self):
        self.rejects("src/services/phase4-types.ts", "membership_date: string;", "membership_date: number;", "VariationStep.dates.membership_date")

    def test_generic_save_record_drift(self):
        self.rejects("src/services/registry-types.ts", "expected_revision_id: string | null;", "expected_revision_id: number | null;", "command save_entity arguments.input.expected_revision_id")

    def test_alias_dto_drift(self):
        self.rejects("src/services/debt-types.ts", "interface PostDebtLinkInput { line_id: string;", "interface PostDebtLinkInput { line_id: number;", "PostDebtLinkInput.line_id")

    def test_serde_field_rename_drift(self):
        self.rejects("crates/domain/src/lib.rs", "pub workspace_name: String,", '#[serde(rename = "workspaceName")]\n    pub workspace_name: String,', "Settings: field keys differ")

    def test_matching_serde_field_rename(self):
        with self.mutation("crates/domain/src/lib.rs", "pub workspace_name: String,", '#[serde(rename = "workspaceName")]\n    pub workspace_name: String,'):
            with self.mutation("src/services/api.ts", "workspace_name: string;", "workspaceName: string;"):
                self.assertEqual([], self.check()[0])

    def test_serde_rename_all_drift(self):
        self.rejects("crates/domain/src/lib.rs", "pub struct Settings {", '#[serde(rename_all = "camelCase")]\npub struct Settings {', "Settings: field keys differ")

    def test_serde_rename_rules_preserve_acronyms(self):
        self.assertEqual("xMLValue", contracts.renamed("XMLValue", {"rename_all": "camelCase"}, enum=True))
        self.assertEqual("x_m_l_value", contracts.renamed("XMLValue", {"rename_all": "snake_case"}, enum=True))
        self.assertEqual("someXMLValue", contracts.renamed("some_XML_value", {"rename_all": "camelCase"}))

    def test_serde_default_does_not_make_emission_optional(self):
        with self.mutation("crates/domain/src/lib.rs", "pub workspace_name: String,", "#[serde(default)]\n    pub workspace_name: String,"):
            self.rejects("src/services/api.ts", "workspace_name: string;", "workspace_name?: string;", "required/optional differs")

    def test_serde_flatten_fails_closed(self):
        self.rejects("crates/domain/src/lib.rs", "pub settings: Settings,", "#[serde(flatten)]\n    pub settings: Settings,", "unsupported Serde attribute flatten")

    def test_custom_serializer_fails_closed(self):
        self.rejects("crates/domain/src/lib.rs", "pub workspace_name: String,", '#[serde(serialize_with = "custom")]\n    pub workspace_name: String,', "unsupported Serde attribute serialize_with")

    def test_serde_enum_rename_drift(self):
        self.rejects("crates/domain/src/lib.rs", '#[serde(rename = "conflict")]', '#[serde(rename = "conflict_v2")]', "ErrorCode: Serde enum values")

    def test_ts_enum_variant_added(self):
        self.rejects("src/services/documents-types.ts", "'active' | 'archived' | 'deleted'", "'active' | 'archived' | 'deleted' | 'pending'", "DocumentRecord.status")

    def test_ts_enum_variant_removed(self):
        self.rejects("src/services/documents-types.ts", "'active' | 'archived' | 'deleted'", "'active' | 'archived'", "DocumentRecord.status")

    def test_rust_enum_variant_added(self):
        self.rejects("crates/domain/src/documents.rs", "pub enum DocumentStatus {", "pub enum DocumentStatus {\n    Pending,", "DocumentRecord.status")

    def test_rust_enum_variant_removed(self):
        self.rejects("crates/domain/src/documents.rs", "    Deleted,\n", "", "DocumentRecord.status")

    def test_enum_cannot_widen_to_unvalidated_string(self):
        self.rejects("src/services/documents-types.ts", "status: DocumentStatus;", "status: string;", "DocumentRecord.status")

    def test_record_macro_expansion_change_fails_closed(self):
        self.rejects("crates/domain/src/registry.rs", "pub revision_id: String,", "pub revision_id: Option<String>,", "record macro definitions changed")

    def test_command_argument_key_drift(self):
        self.rejects("src/services/api.ts", "'list_registry_at', { knowledgeSequence }", "'list_registry_at', { knowledge_sequence: knowledgeSequence }", "command list_registry_at arguments: field keys differ")

    def test_command_argument_type_drift(self):
        self.rejects("src/services/api.ts", "'list_registry_at', { knowledgeSequence }", "'list_registry_at', { knowledgeSequence: 'wrong' }", "command list_registry_at arguments.knowledgeSequence")

    def test_command_result_drift(self):
        self.rejects("src/services/api.ts", "invoke<VaultStatus>('get_status')", "invoke<Settings>('get_status')", "command get_status result")

    def test_unregistered_command(self):
        self.rejects("src-tauri/src/main.rs", "            get_status,", "", "Tauri registry differ")

    def test_wrapper_name_drift(self):
        self.rejects("src/services/api.ts", "'get_status'", "'get_status_v2'", "Tauri registry differ")

    def test_duplicate_wrapper(self):
        self.rejects("src/services/api.ts", "  getStatus: () =>", "  duplicateStatus: () => invoke<VaultStatus>('get_status'),\n  getStatus: () =>", "invoke wrappers: duplicate command")


if __name__ == "__main__":
    unittest.main(verbosity=2)
