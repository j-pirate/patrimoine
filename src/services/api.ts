import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import type { Asset, CompanyPolicy, ConsolidationReport, ConsolidationRequest, Entity, Membership, Ownership, Participation, RegistrySnapshot, SaveRecord, Valuation } from './registry-types';
import type { ExchangeRate, FxQuery, FxResult, ProfilePeriod, RegistryHistory, VariationReport, VariationRequest } from './phase4-types';
import type { DisposePropertyInput, Property, PropertyEvent, PropertyReport, PropertyReportRequest, PropertySnapshot, RentalUnit } from './property-types';
import type { DebtAssetLink, DebtContract, DebtCoverage, DebtGuarantee, DebtMovement, DebtPaymentAllocation, DebtReceivableAdjustment, DebtReport, DebtReportRequest, DebtSchedule, DebtSnapshot, DebtTerms, PostDebtMovementInput } from './debt-types';
import type { AccountReconciliation, CashLink, DepositMovement, PostDepositInput, PostRentalExpenseInput, PostRentPaymentInput, TreasuryAccount, TreasuryReport, TreasuryReportRequest, TreasurySnapshot, TreasuryTransaction } from './treasury-types';
import type { FinanceCoverage, FinanceSnapshot, FinancialReport, FinancialReportRequest, RentClaim, RentClaimAdjustment } from './finance-types';
import type { GenerateRentDuesInput, Lease, LeaseTerms, RefundRentPaymentInput, RentAllocation, RentDue, RentGenerationResult, RentalExpense, RentalReport, RentalReportRequest, RentalSnapshot, RentPayment } from './rental-types';
import type { CompanyDistribution, CompanyFinancialPeriod, CompanyReport, CompanyReportRequest, CompanySnapshot, PostCompanyDistributionInput } from './companies-types';
import type { MetalCashLink, MetalLot, MetalMovement, MetalPremium, MetalQuote, MetalReport, MetalReportRequest, MetalSnapshot, PostMetalMovementInput, TransferMetalOwnershipInput } from './metals-types';
import type { DocumentLibrarySnapshot, DocumentLink, DocumentRecord, ReplaceDocumentInput, SaveDocumentLinkInput, SaveDocumentMetadataInput } from './documents-types';
import type { ExportSummary } from './portability-types';
import type { DashboardReport, DashboardRequest } from './dashboard-types';
import type { GuidedAccountInput, GuidedCompanyInput, GuidedFamilyInput, GuidedMetalInput, GuidedPropertyInput, GuidedRecord } from './guided-types';

export const displayCurrencies = ['EUR', 'TND', 'CHF', 'AED', 'USD'] as const;
export type DisplayCurrency = (typeof displayCurrencies)[number];

export interface Settings {
  workspace_name: string;
  display_currency: DisplayCurrency;
}

export interface VaultStatus {
  generation_id: string;
  data_dir: string;
  schema_version: number;
  app_version: string;
  document_count: number;
  mutation_sequence: number;
  settings: Settings;
  country_count: number;
  currency_count: number;
  recovery_required: boolean;
  recovery_message: string | null;
}

export interface DocumentSummary {
  id: string;
  original_name: string;
  mime_type: string;
  size_bytes: number;
  sha256: string;
  added_at: string;
}

export interface BackupSummary {
  path: string;
  sha256: string;
  size_bytes: number;
  document_count: number;
  blob_count: number;
  created_at: string;
}

export interface RestoreInspection {
  token: string;
  archive_sha256: string;
  created_at: string;
  schema_version: number;
  app_version: string;
  document_count: number;
  blob_count: number;
  settings: Settings;
  source_generation_id: string;
  current_generation_id: string;
  mutation_sequence: number;
}

export const errorCodes = [
  'conflict', 'validation', 'io', 'database', 'schema_version', 'integrity', 'not_found',
  'invalid_archive', 'invalid_csv', 'publication_unavailable',
  'publication_uncertain', 'recovery_required', 'recovery', 'locked', 'lock_io',
  'capacity', 'limit', 'confirmation_required', 'snapshot_timeout', 'document_open',
  'unsupported_document_open', 'xlsx_export', 'FILE_SCOPE', 'VAULT_LOCK', 'TASK_FAILED',
] as const;
export type ErrorCode = (typeof errorCodes)[number];

export function errorCode(error: unknown): ErrorCode | null {
  if (typeof error !== 'object' || error === null || !('code' in error)) return null;
  const code = (error as { code: unknown }).code;
  return typeof code === 'string' && (errorCodes as readonly string[]).includes(code) ? code as ErrorCode : null;
}

export function errorAction(error: unknown): string | null {
  switch (errorCode(error)) {
    case 'conflict': return 'Conservez votre saisie, rechargez les données puis comparez les révisions avant de réenregistrer.';
    case 'validation': return 'Vérifiez les champs, dates, devises et références indiqués.';
    case 'FILE_SCOPE': return 'Sélectionnez de nouveau le fichier avec le dialogue de l’application.';
    case 'io': case 'lock_io': case 'publication_uncertain':
      return 'Vérifiez le support et l’espace disponible. Contrôlez le fichier ou l’état du coffre avant de recommencer.';
    case 'database':
      return 'Fermez les outils qui accèdent au coffre, puis réessayez.';
    case 'schema_version':
      return 'Vérifiez la version de Patrimoine compatible avec ce coffre ou cette archive.';
    case 'invalid_csv':
      return 'Vérifiez la table, la ligne et la colonne indiquées dans le diagnostic.';
    case 'recovery_required': case 'recovery': case 'VAULT_LOCK': case 'TASK_FAILED':
      return 'Consultez l’état du coffre et redémarrez l’application si nécessaire, en conservant ses données.';
    case 'snapshot_timeout': return 'Arrêtez les écritures concurrentes puis relancez la sauvegarde.';
    case 'invalid_archive': return 'Choisissez une archive compatible et intacte ; conservez l’original pour le diagnostic.';
    default: return null;
  }
}

export function errorMessage(error: unknown): string {
  let message = 'Une erreur est survenue. Réessayez ; aucune réussite n’a été confirmée.';
  if (typeof error === 'object' && error !== null && 'message' in error) {
    const supplied = (error as { message: unknown }).message;
    if (typeof supplied === 'string' && supplied.length > 0) message = supplied;
  } else if (typeof error === 'string' && error.length > 0) message = error;
  const action = errorAction(error);
  return action && !message.includes(action) ? `${message} ${action}` : message;
}

const zipFilter = [{ name: 'Sauvegarde Patrimoine', extensions: ['zip'] }];

export const api = {
  onDocumentOpenFailed: (onError: (error: { code: string; message: string }) => void) => listen<{ code: string; message: string }>('document-open-failed', event => onError(event.payload)),
  getStatus: () => invoke<VaultStatus>('get_status'),
  getSettings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  listRegistry: () => invoke<RegistrySnapshot>('list_registry'),
  listRegistryAt: (knowledgeSequence: number | null) => invoke<RegistrySnapshot>('list_registry_at', { knowledgeSequence }),
  listRegistryHistory: () => invoke<RegistryHistory>('list_registry_history'),
  readOnboardingState: (generationId: string) => invoke<string | null>('read_onboarding_state', { generationId }),
  writeOnboardingState: (generationId: string, data: string) => invoke<void>('write_onboarding_state', { generationId, data }),
  createGuidedFamily: (input: GuidedFamilyInput) => invoke<GuidedRecord>('create_guided_family', { input }),
  createGuidedProperty: (input: GuidedPropertyInput) => invoke<GuidedRecord>('create_guided_property', { input }),
  createGuidedAccount: (input: GuidedAccountInput) => invoke<GuidedRecord>('create_guided_account', { input }),
  createGuidedMetal: (input: GuidedMetalInput) => invoke<GuidedRecord>('create_guided_metal', { input }),
  createGuidedCompany: (input: GuidedCompanyInput) => invoke<GuidedRecord>('create_guided_company', { input }),
  saveExchangeRate: (input: SaveRecord<ExchangeRate>) => invoke<ExchangeRate>('save_exchange_rate', { input }),
  saveProfilePeriod: (input: SaveRecord<ProfilePeriod>) => invoke<ProfilePeriod>('save_profile_period', { input }),
  convertCurrency: (query: FxQuery) => invoke<FxResult>('convert_currency', { query }),
  explainVariation: (request: VariationRequest) => invoke<VariationReport>('explain_variation', { request }),
  saveEntity: (input: SaveRecord<Entity>) => invoke<Entity>('save_entity', { input }),
  saveMembership: (input: SaveRecord<Membership>) => invoke<Membership>('save_membership', { input }),
  saveParticipation: (input: SaveRecord<Participation>) => invoke<Participation>('save_participation', { input }),
  saveAsset: (input: SaveRecord<Asset>) => invoke<Asset>('save_asset', { input }),
  saveOwnership: (input: SaveRecord<Ownership>) => invoke<Ownership>('save_ownership', { input }),
  saveValuation: (input: SaveRecord<Valuation>) => invoke<Valuation>('save_valuation', { input }),
  saveCompanyPolicy: (input: SaveRecord<CompanyPolicy>) => invoke<CompanyPolicy>('save_company_policy', { input }),
  voidRegistryRecord: (id: string, expectedRevisionId: string, reason: string) =>
    invoke<void>('void_registry_record', { input: { id, expected_revision_id: expectedRevisionId, reason } }),
  getConsolidation: (request: ConsolidationRequest) => invoke<ConsolidationReport>('get_consolidation', { request }),
  getDashboard: (request: DashboardRequest) => invoke<DashboardReport>('get_dashboard', { request }),
  exportDashboardReport: (request: DashboardRequest, destinationPath: string) => invoke<void>('export_dashboard_report', { request, destinationPath }),
  chooseDashboardReportDestination: () => save({ title: 'Exporter le rapport filtré', defaultPath: `patrimoine-rapport-${new Date().toISOString().slice(0, 10)}.json`, filters: [{ name: 'Rapport patrimonial JSON', extensions: ['json'] }] }),
  listPropertiesAt: (knowledgeSequence: number | null) => invoke<PropertySnapshot>('list_properties_at', { knowledgeSequence }),
  saveProperty: (input: SaveRecord<Property>) => invoke<Property>('save_property', { input }),
  saveRentalUnit: (input: SaveRecord<RentalUnit>) => invoke<RentalUnit>('save_rental_unit', { input }),
  savePropertyEvent: (input: SaveRecord<PropertyEvent>) => invoke<PropertyEvent>('save_property_event', { input }),
  disposeProperty: (input: DisposePropertyInput) => invoke<PropertySnapshot>('dispose_property', { input }),
  getPropertyReport: (request: PropertyReportRequest) => invoke<PropertyReport>('get_property_report', { request }),
  listRentalsAt: (knowledgeSequence: number | null) => invoke<RentalSnapshot>('list_rentals_at', { knowledgeSequence }),
  saveLease: (input: SaveRecord<Lease>) => invoke<Lease>('save_lease', { input }),
  saveLeaseTerms: (input: SaveRecord<LeaseTerms>) => invoke<LeaseTerms>('save_lease_terms', { input }),
  saveRentDue: (input: SaveRecord<RentDue>) => invoke<RentDue>('save_rent_due', { input }),
  saveRentPayment: (input: SaveRecord<RentPayment>) => invoke<RentPayment>('save_rent_payment', { input }),
  refundRentPayment: (input: RefundRentPaymentInput) => invoke<RentPayment>('refund_rent_payment', { input }),
  saveRentAllocation: (input: SaveRecord<RentAllocation>) => invoke<RentAllocation>('save_rent_allocation', { input }),
  saveRentalExpense: (input: SaveRecord<RentalExpense>) => invoke<RentalExpense>('save_rental_expense', { input }),
  generateRentDues: (input: GenerateRentDuesInput) => invoke<RentGenerationResult>('generate_rent_dues', { input }),
  getRentalReport: (request: RentalReportRequest) => invoke<RentalReport>('get_rental_report', { request }),
  listDebtsAt: (knowledgeSequence: number | null) => invoke<DebtSnapshot>('list_debts_at', { knowledgeSequence }),
  saveDebtContract: (input: SaveRecord<DebtContract>) => invoke<DebtContract>('save_debt_contract', { input }),
  saveDebtTerms: (input: SaveRecord<DebtTerms>) => invoke<DebtTerms>('save_debt_terms', { input }),
  saveDebtMovement: (input: SaveRecord<DebtMovement>) => invoke<DebtMovement>('save_debt_movement', { input }),
  saveDebtReceivableAdjustment: (input: SaveRecord<DebtReceivableAdjustment>) => invoke<DebtReceivableAdjustment>('save_debt_receivable_adjustment', { input }),
  saveDebtSchedule: (input: SaveRecord<DebtSchedule>) => invoke<DebtSchedule>('save_debt_schedule', { input }),
  saveDebtPaymentAllocation: (input: SaveRecord<DebtPaymentAllocation>) => invoke<DebtPaymentAllocation>('save_debt_payment_allocation', { input }),
  saveDebtAssetLink: (input: SaveRecord<DebtAssetLink>) => invoke<DebtAssetLink>('save_debt_asset_link', { input }),
  saveDebtGuarantee: (input: SaveRecord<DebtGuarantee>) => invoke<DebtGuarantee>('save_debt_guarantee', { input }),
  saveDebtCoverage: (input: SaveRecord<DebtCoverage>) => invoke<DebtCoverage>('save_debt_coverage', { input }),
  postDebtMovement: (input: PostDebtMovementInput) => invoke<DebtMovement>('post_debt_movement', { input }),
  getDebtReport: (request: DebtReportRequest) => invoke<DebtReport>('get_debt_report', { request }),
  listTreasuryAt: (knowledgeSequence: number | null) => invoke<TreasurySnapshot>('list_treasury_at', { knowledgeSequence }),
  saveTreasuryAccount: (input: SaveRecord<TreasuryAccount>) => invoke<TreasuryAccount>('save_treasury_account', { input }),
  saveTreasuryTransaction: (input: SaveRecord<TreasuryTransaction>) => invoke<TreasuryTransaction>('save_treasury_transaction', { input }),
  saveAccountReconciliation: (input: SaveRecord<AccountReconciliation>) => invoke<AccountReconciliation>('save_account_reconciliation', { input }),
  saveCashLink: (input: SaveRecord<CashLink>) => invoke<CashLink>('save_cash_link', { input }),
  saveDepositMovement: (input: SaveRecord<DepositMovement>) => invoke<DepositMovement>('save_deposit_movement', { input }),
  postRentPayment: (input: PostRentPaymentInput) => invoke<RentPayment>('post_rent_payment', { input }),
  postRentalExpense: (input: PostRentalExpenseInput) => invoke<RentalExpense>('post_rental_expense', { input }),
  postDeposit: (input: PostDepositInput) => invoke<DepositMovement>('post_deposit', { input }),
  getTreasuryReport: (request: TreasuryReportRequest) => invoke<TreasuryReport>('get_treasury_report', { request }),
  listFinanceAt: (knowledgeSequence: number | null) => invoke<FinanceSnapshot>('list_finance_at', { knowledgeSequence }),
  saveFinanceCoverage: (input: SaveRecord<FinanceCoverage>) => invoke<FinanceCoverage>('save_finance_coverage', { input }),
  saveRentClaim: (input: SaveRecord<RentClaim>) => invoke<RentClaim>('save_rent_claim', { input }),
  saveRentClaimAdjustment: (input: SaveRecord<RentClaimAdjustment>) => invoke<RentClaimAdjustment>('save_rent_claim_adjustment', { input }),
  getFinancialReport: (request: FinancialReportRequest) => invoke<FinancialReport>('get_financial_report', { request }),
  listCompaniesAt: (knowledgeSequence: number | null) => invoke<CompanySnapshot>('list_companies_at', { knowledgeSequence }),
  saveCompanyFinancialPeriod: (input: SaveRecord<CompanyFinancialPeriod>) => invoke<CompanyFinancialPeriod>('save_company_financial_period', { input }),
  saveCompanyDistribution: (input: SaveRecord<CompanyDistribution>) => invoke<CompanyDistribution>('save_company_distribution', { input }),
  postCompanyDistribution: (input: PostCompanyDistributionInput) => invoke<CompanyDistribution>('post_company_distribution', { input }),
  getCompanyReport: (request: CompanyReportRequest) => invoke<CompanyReport>('get_company_report', { request }),
  listMetalsAt: (knowledgeSequence: number | null) => invoke<MetalSnapshot>('list_metals_at', { knowledgeSequence }),
  saveMetalLot: (input: SaveRecord<MetalLot>) => invoke<MetalLot>('save_metal_lot', { input }),
  saveMetalMovement: (input: SaveRecord<MetalMovement>) => invoke<MetalMovement>('save_metal_movement', { input }),
  saveMetalQuote: (input: SaveRecord<MetalQuote>) => invoke<MetalQuote>('save_metal_quote', { input }),
  saveMetalPremium: (input: SaveRecord<MetalPremium>) => invoke<MetalPremium>('save_metal_premium', { input }),
  saveMetalCashLink: (input: SaveRecord<MetalCashLink>) => invoke<MetalCashLink>('save_metal_cash_link', { input }),
  postMetalMovement: (input: PostMetalMovementInput) => invoke<MetalMovement>('post_metal_movement', { input }),
  transferMetalOwnership: (input: TransferMetalOwnershipInput) => invoke<RegistrySnapshot>('transfer_metal_ownership', { input }),
  getMetalReport: (request: MetalReportRequest) => invoke<MetalReport>('get_metal_report', { request }),
  listDocuments: () => invoke<DocumentSummary[]>('list_documents'),
  listDocumentLibraryAt: (knowledgeSequence: number | null) => invoke<DocumentLibrarySnapshot>('list_document_library_at', { knowledgeSequence }),
  saveDocumentMetadata: (input: SaveDocumentMetadataInput) => invoke<DocumentRecord>('save_document_metadata', { input }),
  replaceDocument: (input: ReplaceDocumentInput, sourcePath: string) => invoke<DocumentRecord>('replace_document', { input, sourcePath }),
  saveDocumentLink: (input: SaveDocumentLinkInput) => invoke<DocumentLink>('save_document_link', { input }),
  removeDocumentLink: (id: string, expectedRevisionId: string, reason: string) => invoke<void>('remove_document_link', { input: { id, expected_revision_id: expectedRevisionId, reason } }),
  exportDocumentRevision: (documentId: string, revisionId: string, destinationPath: string) => invoke<void>('export_document_revision', { documentId, revisionId, destinationPath }),
  openDocumentRevision: (documentId: string, revisionId: string) => invoke<void>('open_document_revision', { documentId, revisionId }),
  importDocument: (sourcePath: string, description = '') =>
    invoke<DocumentSummary>('import_document', { sourcePath, description }),
  exportDocument: (documentId: string, destinationPath: string) =>
    invoke<void>('export_document', { documentId, destinationPath }),
  createBackup: (destinationPath: string) => invoke<BackupSummary>('create_backup', { destinationPath }),
  inspectRestore: (archivePath: string) => invoke<RestoreInspection>('inspect_restore', { archivePath }),
  exportCsvArchive: (destinationPath: string) => invoke<ExportSummary>('export_csv_archive', { destinationPath }),
  exportXlsx: (destinationPath: string) => invoke<ExportSummary>('export_xlsx', { destinationPath }),
  inspectCsvImport: (archivePath: string) => invoke<RestoreInspection>('inspect_csv_import', { archivePath }),
  activateRestore: (token: string, safetyBackupPath: string | null) =>
    invoke<VaultStatus>('activate_restore', { token, confirmed: true, safetyBackupPath }),
  chooseBackupDestination: (safety = false) =>
    save({
      title: safety ? 'Sauvegarder l’état actuel avant restauration' : 'Enregistrer une sauvegarde complète',
      defaultPath: `patrimoine-${safety ? 'avant-restauration-' : ''}${new Date().toISOString().slice(0, 10)}.zip`,
      filters: zipFilter,
    }),
  chooseRestoreArchive: () =>
    open({ title: 'Analyser une sauvegarde Patrimoine', multiple: false, directory: false, filters: zipFilter }),
  chooseDocumentImport: () =>
    open({ title: 'Importer un document', multiple: false, directory: false }),
  chooseDocumentReplacement: () =>
    open({ title: 'Remplacer l’original du document', multiple: false, directory: false }),
  chooseDocumentDestination: (originalName: string) =>
    save({ title: 'Exporter une copie du document', defaultPath: originalName.replace(/[/\\]/g, '_') }),
  chooseCsvExportDestination: () => save({ title: 'Exporter une archive CSV intégrale', defaultPath: `patrimoine-csv-${new Date().toISOString().slice(0, 10)}.zip`, filters: [{ name: 'Archive CSV Patrimoine', extensions: ['zip'] }] }),
  chooseXlsxDestination: () => save({ title: 'Exporter un classeur XLSX complet', defaultPath: `patrimoine-${new Date().toISOString().slice(0, 10)}.xlsx`, filters: [{ name: 'Classeur de consultation', extensions: ['xlsx'] }] }),
  chooseCsvImportArchive: () => open({ title: 'Analyser une archive CSV Patrimoine', multiple: false, directory: false, filters: [{ name: 'Archive CSV Patrimoine', extensions: ['zip'] }] }),
};
