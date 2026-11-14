//! Physical inventory and actual cash remain distinct, versioned sources.
use patrimoine_domain::*;

pub trait MetalService {
    fn list_metals_at(&self, knowledge_sequence: Option<u64>) -> Result<MetalSnapshot>;
    fn save_metal_lot(&mut self, input: SaveMetalLotInput) -> Result<MetalLot>;
    fn save_metal_movement(&mut self, input: SaveMetalMovementInput) -> Result<MetalMovement>;
    fn save_metal_quote(&mut self, input: SaveMetalQuoteInput) -> Result<MetalQuote>;
    fn save_metal_premium(&mut self, input: SaveMetalPremiumInput) -> Result<MetalPremium>;
    fn save_metal_cash_link(&mut self, input: SaveMetalCashLinkInput) -> Result<MetalCashLink>;
    fn post_metal_movement(&mut self, input: PostMetalMovementInput) -> Result<MetalMovement>;
    fn transfer_metal_ownership(
        &mut self,
        input: TransferMetalOwnershipInput,
    ) -> Result<RegistrySnapshot>;
    fn get_metal_report(&self, request: MetalReportRequest) -> Result<MetalReport>;
}
