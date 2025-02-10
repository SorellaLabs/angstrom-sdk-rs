use alloy_primitives::{aliases::U40, Address, Bytes, PrimitiveSignature, B256, U256};
use angstrom_rpc::api::GasEstimateResponse;
use angstrom_sdk_rs_macros::{neon_object_as, NeonObject};
use angstrom_types::{
    orders::{CancelOrderRequest, OrderLocation, OrderStatus},
    primitive::OrderPoolNewOrderResult,
    sol_bindings::{
        grouped_orders::{AllOrders, FlashVariants, StandingVariants},
        rpc_orders::{
            ExactFlashOrder, ExactStandingOrder, OrderMeta, PartialFlashOrder,
            PartialStandingOrder, TopOfBlockOrder
        }
    }
};
use neon::object::Object;
use pade::PadeEncode;

#[derive(Debug, Clone, NeonObject)]
pub struct AllOrdersWithSignature {
    order:          AllOrders,
    order_type:     AllOrderType,
    signer_address: Address,
    signature:      Bytes
}

impl Into<AllOrders> for AllOrdersWithSignature {
    fn into(self) -> AllOrders {
        match self.order {
            AllOrders::Standing(standing_variants) => match standing_variants {
                StandingVariants::Partial(mut partial_standing_order) => {
                    partial_standing_order.meta = OrderMeta {
                        isEcdsa:   true,
                        from:      self.signer_address,
                        signature: PrimitiveSignature::try_from(&**self.signature)
                            .unwrap()
                            .pade_encode()
                            .into()
                    };
                    AllOrders::Standing(StandingVariants::Partial(partial_standing_order))
                }
                StandingVariants::Exact(mut exact_standing_order) => {
                    exact_standing_order.meta = OrderMeta {
                        isEcdsa:   true,
                        from:      self.signer_address,
                        signature: PrimitiveSignature::try_from(&**self.signature)
                            .unwrap()
                            .pade_encode()
                            .into()
                    };
                    AllOrders::Standing(StandingVariants::Exact(exact_standing_order))
                }
            },
            AllOrders::Flash(flash_variants) => match flash_variants {
                FlashVariants::Partial(mut partial_flash_order) => {
                    partial_flash_order.meta = OrderMeta {
                        isEcdsa:   true,
                        from:      self.signer_address,
                        signature: PrimitiveSignature::try_from(&**self.signature)
                            .unwrap()
                            .pade_encode()
                            .into()
                    };
                    AllOrders::Flash(FlashVariants::Partial(partial_flash_order))
                }
                FlashVariants::Exact(mut exact_flash_order) => {
                    exact_flash_order.meta = OrderMeta {
                        isEcdsa:   true,
                        from:      self.signer_address,
                        signature: PrimitiveSignature::try_from(&**self.signature)
                            .unwrap()
                            .pade_encode()
                            .into()
                    };
                    AllOrders::Flash(FlashVariants::Exact(exact_flash_order))
                }
            },
            AllOrders::TOB(mut top_of_block_order) => {
                top_of_block_order.meta = OrderMeta {
                    isEcdsa:   true,
                    from:      self.signer_address,
                    signature: PrimitiveSignature::try_from(&**self.signature)
                        .unwrap()
                        .pade_encode()
                        .into()
                };
                AllOrders::TOB(top_of_block_order)
            }
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
pub enum AllOrdersNeon {
    Standing { order: StandingVariantsNeon },
    Flash { order: FlashVariantsNeon },
    TOB { order: TopOfBlockOrderSolBindingsNeon }
}

impl From<AllOrders> for AllOrdersNeon {
    fn from(value: AllOrders) -> Self {
        match value {
            AllOrders::Standing(standing_variants) => {
                AllOrdersNeon::Standing { order: standing_variants.into() }
            }
            AllOrders::Flash(flash_variants) => {
                AllOrdersNeon::Flash { order: flash_variants.into() }
            }
            AllOrders::TOB(top_of_block_order) => {
                AllOrdersNeon::TOB { order: top_of_block_order.into() }
            }
        }
    }
}

impl Into<AllOrders> for AllOrdersNeon {
    fn into(self) -> AllOrders {
        match self {
            Self::Standing { order } => AllOrders::Standing(order.into()),
            Self::Flash { order } => AllOrders::Flash(order.into()),
            Self::TOB { order } => AllOrders::TOB(order.into())
        }
    }
}

neon_object_as!(AllOrders, AllOrdersNeon);

#[derive(Debug, Clone, NeonObject)]
enum StandingVariantsNeon {
    Partial { order: PartialStandingOrderNeon },
    Exact { order: ExactStandingOrderNeon }
}

impl From<StandingVariants> for StandingVariantsNeon {
    fn from(value: StandingVariants) -> Self {
        match value {
            StandingVariants::Partial(order) => Self::Partial { order: order.into() },
            StandingVariants::Exact(order) => Self::Exact { order: order.into() }
        }
    }
}

impl Into<StandingVariants> for StandingVariantsNeon {
    fn into(self) -> StandingVariants {
        match self {
            Self::Partial { order } => StandingVariants::Partial(order.into()),
            Self::Exact { order } => StandingVariants::Exact(order.into())
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct PartialStandingOrderNeon {
    ref_id:               u32,
    min_amount_in:        u128,
    max_amount_in:        u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    nonce:                u64,
    deadline:             u64 // meta:                 OrderMetaNeon
}

impl From<PartialStandingOrder> for PartialStandingOrderNeon {
    fn from(value: PartialStandingOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            min_amount_in:        value.min_amount_in,
            max_amount_in:        value.max_amount_in,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            nonce:                value.nonce,
            deadline:             value.deadline.to() // meta:                 value.meta.into()
        }
    }
}

impl Into<PartialStandingOrder> for PartialStandingOrderNeon {
    fn into(self) -> PartialStandingOrder {
        PartialStandingOrder {
            ref_id:               self.ref_id,
            min_amount_in:        self.min_amount_in,
            max_amount_in:        self.max_amount_in,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            nonce:                self.nonce,
            deadline:             U40::from(self.deadline),
            meta:                 Default::default()
        }
    }
}

neon_object_as!(PartialStandingOrder, PartialStandingOrderNeon);

#[derive(Debug, Clone, NeonObject)]
pub struct ExactStandingOrderNeon {
    ref_id:               u32,
    exact_in:             bool,
    amount:               u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    nonce:                u64,
    deadline:             u64 // meta:                 OrderMetaNeon
}

impl From<ExactStandingOrder> for ExactStandingOrderNeon {
    fn from(value: ExactStandingOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            nonce:                value.nonce,
            deadline:             value.deadline.to(),
            // meta:                 value.meta.into(),
            exact_in:             value.exact_in,
            amount:               value.amount
        }
    }
}

impl Into<ExactStandingOrder> for ExactStandingOrderNeon {
    fn into(self) -> ExactStandingOrder {
        ExactStandingOrder {
            ref_id:               self.ref_id,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            nonce:                self.nonce,
            deadline:             U40::from(self.deadline),
            meta:                 Default::default(),
            exact_in:             self.exact_in,
            amount:               self.amount
        }
    }
}

neon_object_as!(ExactStandingOrder, ExactStandingOrderNeon);

#[derive(Debug, Clone, NeonObject)]
enum FlashVariantsNeon {
    Partial { order: PartialFlashOrderNeon },
    Exact { order: ExactFlashOrderNeon }
}

impl From<FlashVariants> for FlashVariantsNeon {
    fn from(value: FlashVariants) -> Self {
        match value {
            FlashVariants::Partial(order) => Self::Partial { order: order.into() },
            FlashVariants::Exact(order) => Self::Exact { order: order.into() }
        }
    }
}

impl Into<FlashVariants> for FlashVariantsNeon {
    fn into(self) -> FlashVariants {
        match self {
            Self::Partial { order } => FlashVariants::Partial(order.into()),
            Self::Exact { order } => FlashVariants::Exact(order.into())
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct PartialFlashOrderNeon {
    ref_id:               u32,
    min_amount_in:        u128,
    max_amount_in:        u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    valid_for_block:      u64 // meta:                 OrderMetaNeon
}

impl From<PartialFlashOrder> for PartialFlashOrderNeon {
    fn from(value: PartialFlashOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            min_amount_in:        value.min_amount_in,
            max_amount_in:        value.max_amount_in,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            valid_for_block:      value.valid_for_block // meta:                 value.meta.into()
        }
    }
}

impl Into<PartialFlashOrder> for PartialFlashOrderNeon {
    fn into(self) -> PartialFlashOrder {
        PartialFlashOrder {
            ref_id:               self.ref_id,
            min_amount_in:        self.min_amount_in,
            max_amount_in:        self.max_amount_in,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            valid_for_block:      self.valid_for_block,
            // meta:                 self.meta.into()
            meta:                 Default::default()
        }
    }
}

neon_object_as!(PartialFlashOrder, PartialFlashOrderNeon);

#[derive(Debug, Clone, NeonObject)]
pub struct ExactFlashOrderNeon {
    ref_id:               u32,
    exact_in:             bool,
    amount:               u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    valid_for_block:      u64 // meta:                 OrderMetaNeon
}

impl From<ExactFlashOrder> for ExactFlashOrderNeon {
    fn from(value: ExactFlashOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            valid_for_block:      value.valid_for_block,
            // meta:                 value.meta.into(),
            exact_in:             value.exact_in,
            amount:               value.amount
        }
    }
}

impl Into<ExactFlashOrder> for ExactFlashOrderNeon {
    fn into(self) -> ExactFlashOrder {
        ExactFlashOrder {
            ref_id:               self.ref_id,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            valid_for_block:      self.valid_for_block,
            // meta:                 self.meta.into(),
            meta:                 Default::default(),
            exact_in:             self.exact_in,
            amount:               self.amount
        }
    }
}

neon_object_as!(ExactFlashOrder, ExactFlashOrderNeon);

#[derive(Debug, Clone, NeonObject)]
pub struct TopOfBlockOrderSolBindingsNeon {
    quantity_in:     u128,
    quantity_out:    u128,
    max_gas_asset0:  u128,
    use_internal:    bool,
    asset_in:        Address,
    asset_out:       Address,
    recipient:       Address,
    valid_for_block: u64 // meta:            OrderMetaNeon
}

impl From<TopOfBlockOrder> for TopOfBlockOrderSolBindingsNeon {
    fn from(value: TopOfBlockOrder) -> Self {
        Self {
            quantity_in:     value.quantity_in,
            quantity_out:    value.quantity_out,
            max_gas_asset0:  value.max_gas_asset0,
            use_internal:    value.use_internal,
            asset_in:        value.asset_in,
            asset_out:       value.asset_out,
            recipient:       value.recipient,
            valid_for_block: value.valid_for_block // meta:            value.meta.into()
        }
    }
}

impl Into<TopOfBlockOrder> for TopOfBlockOrderSolBindingsNeon {
    fn into(self) -> TopOfBlockOrder {
        TopOfBlockOrder {
            quantity_in:     self.quantity_in,
            quantity_out:    self.quantity_out,
            max_gas_asset0:  self.max_gas_asset0,
            use_internal:    self.use_internal,
            asset_in:        self.asset_in,
            asset_out:       self.asset_out,
            recipient:       self.recipient,
            valid_for_block: self.valid_for_block,
            // meta:            self.meta.into()
            meta:            Default::default()
        }
    }
}

neon_object_as!(TopOfBlockOrder, TopOfBlockOrderSolBindingsNeon);

// #[allow(non_snake_case)]
// #[derive(Debug, Clone, NeonObject)]
// struct OrderMetaNeon {
//     isEcdsa:   bool,
//     from:      Address,
//     signature: Bytes
// }

// impl From<OrderMeta> for OrderMetaNeon {
//     fn from(value: OrderMeta) -> Self {
//         Self { isEcdsa: value.isEcdsa, from: value.from, signature:
// value.signature }     }
// }

// impl Into<OrderMeta> for OrderMetaNeon {
//     fn into(self) -> OrderMeta {
//         OrderMeta { isEcdsa: self.isEcdsa, from: self.from, signature:
// self.signature }     }
// }

#[derive(Debug, Clone, NeonObject)]
pub enum OrderPoolNewOrderResultNeon {
    Valid,
    Invalid,
    TransitionedToBlock,
    Error { error: String }
}

impl From<OrderPoolNewOrderResult> for OrderPoolNewOrderResultNeon {
    fn from(value: OrderPoolNewOrderResult) -> Self {
        match value {
            OrderPoolNewOrderResult::Valid => Self::Valid,
            OrderPoolNewOrderResult::Invalid => Self::Invalid,
            OrderPoolNewOrderResult::TransitionedToBlock => Self::TransitionedToBlock,
            OrderPoolNewOrderResult::Error(error) => OrderPoolNewOrderResultNeon::Error { error }
        }
    }
}

impl Into<OrderPoolNewOrderResult> for OrderPoolNewOrderResultNeon {
    fn into(self) -> OrderPoolNewOrderResult {
        match self {
            OrderPoolNewOrderResultNeon::Valid => OrderPoolNewOrderResult::Valid,
            OrderPoolNewOrderResultNeon::Invalid => OrderPoolNewOrderResult::Invalid,
            OrderPoolNewOrderResultNeon::TransitionedToBlock => {
                OrderPoolNewOrderResult::TransitionedToBlock
            }
            OrderPoolNewOrderResultNeon::Error { error } => OrderPoolNewOrderResult::Error(error)
        }
    }
}

neon_object_as!(OrderPoolNewOrderResult, OrderPoolNewOrderResultNeon);

#[derive(Debug, Clone, NeonObject)]
pub struct CancelOrderRequestNeon {
    signature:    PrimitiveSignatureNeon,
    user_address: Address,
    order_id:     B256
}

impl From<CancelOrderRequest> for CancelOrderRequestNeon {
    fn from(value: CancelOrderRequest) -> Self {
        Self {
            signature:    value.signature.into(),
            user_address: value.user_address,
            order_id:     value.order_id
        }
    }
}

impl Into<CancelOrderRequest> for CancelOrderRequestNeon {
    fn into(self) -> CancelOrderRequest {
        CancelOrderRequest {
            signature:    self.signature.into(),
            user_address: self.user_address,
            order_id:     self.order_id
        }
    }
}

neon_object_as!(CancelOrderRequest, CancelOrderRequestNeon);

#[derive(Debug, Clone, NeonObject)]
struct PrimitiveSignatureNeon {
    y_parity: bool,
    r:        U256,
    s:        U256
}

impl From<PrimitiveSignature> for PrimitiveSignatureNeon {
    fn from(value: PrimitiveSignature) -> Self {
        Self { y_parity: value.v(), r: value.r(), s: value.s() }
    }
}

impl Into<PrimitiveSignature> for PrimitiveSignatureNeon {
    fn into(self) -> PrimitiveSignature {
        PrimitiveSignature::new(self.r, self.s, self.y_parity)
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct GasEstimateResponseNeon {
    gas_units: u64,
    gas:       U256
}

impl From<GasEstimateResponse> for GasEstimateResponseNeon {
    fn from(value: GasEstimateResponse) -> Self {
        Self { gas: value.gas, gas_units: value.gas_units }
    }
}

impl Into<GasEstimateResponse> for GasEstimateResponseNeon {
    fn into(self) -> GasEstimateResponse {
        GasEstimateResponse { gas: self.gas, gas_units: self.gas_units }
    }
}

neon_object_as!(GasEstimateResponse, GasEstimateResponseNeon);

#[derive(Debug, Clone, NeonObject)]
pub enum OrderStatusNeon {
    Blocked,
    Filled,
    Pending
}

impl From<OrderStatus> for OrderStatusNeon {
    fn from(value: OrderStatus) -> Self {
        match value {
            OrderStatus::Blocked => Self::Blocked,
            OrderStatus::Filled => Self::Filled,
            OrderStatus::Pending => Self::Pending
        }
    }
}

impl Into<OrderStatus> for OrderStatusNeon {
    fn into(self) -> OrderStatus {
        match self {
            OrderStatusNeon::Blocked => OrderStatus::Blocked,
            OrderStatusNeon::Filled => OrderStatus::Filled,
            OrderStatusNeon::Pending => OrderStatus::Pending
        }
    }
}

neon_object_as!(OrderStatus, OrderStatusNeon);

#[derive(Debug, Clone, NeonObject)]
pub enum OrderLocationNeon {
    Limit,
    Searcher
}

impl From<OrderLocation> for OrderLocationNeon {
    fn from(value: OrderLocation) -> Self {
        match value {
            OrderLocation::Limit => Self::Limit,
            OrderLocation::Searcher => Self::Searcher
        }
    }
}

impl Into<OrderLocation> for OrderLocationNeon {
    fn into(self) -> OrderLocation {
        match self {
            OrderLocationNeon::Searcher => OrderLocation::Searcher,
            OrderLocationNeon::Limit => OrderLocation::Limit
        }
    }
}

neon_object_as!(OrderLocation, OrderLocationNeon);

#[derive(Debug, Clone, NeonObject)]
pub enum AllOrderType {
    TOB,
    PartialStandingOrder,
    ExactStandingOrder,
    PartialFlashOrder,
    ExactFlashOrder
}
