use angstrom_types::contract_payloads::{
    Asset, Pair,
    angstrom::{AngstromBundle, TopOfBlockOrder, UserOrder}
};

pub trait BundleUnpack {
    fn unpack_user_orders(&self) -> impl Iterator<Item = (&UserOrder, (&Asset, &Asset), &Pair)>;

    fn unpack_tob_orders(
        &self
    ) -> impl Iterator<Item = (&TopOfBlockOrder, (&Asset, &Asset), &Pair)>;
}

impl BundleUnpack for AngstromBundle {
    fn unpack_user_orders(&self) -> impl Iterator<Item = (&UserOrder, (&Asset, &Asset), &Pair)> {
        self.user_orders.iter().map(|o| {
            let pair = &self.pairs[o.pair_index as usize];
            let zfo = o.zero_for_one;
            let asset_in = if zfo {
                &self.assets[pair.index0 as usize]
            } else {
                &self.assets[pair.index1 as usize]
            };
            let asset_out = if !zfo {
                &self.assets[pair.index0 as usize]
            } else {
                &self.assets[pair.index1 as usize]
            };
            let bundle_assets = if asset_in.addr <= asset_out.addr {
                (asset_in, asset_out)
            } else {
                (asset_out, asset_in)
            };
            (o, bundle_assets, pair)
        })
    }

    fn unpack_tob_orders(
        &self
    ) -> impl Iterator<Item = (&TopOfBlockOrder, (&Asset, &Asset), &Pair)> {
        self.top_of_block_orders.iter().map(|o| {
            let pair = &self.pairs[o.pairs_index as usize];
            let zfo = o.zero_for_1;
            let asset_in = if zfo {
                &self.assets[pair.index0 as usize]
            } else {
                &self.assets[pair.index1 as usize]
            };
            let asset_out = if !zfo {
                &self.assets[pair.index0 as usize]
            } else {
                &self.assets[pair.index1 as usize]
            };
            let bundle_assets = if asset_in.addr <= asset_out.addr {
                (asset_in, asset_out)
            } else {
                (asset_out, asset_in)
            };
            (o, bundle_assets, pair)
        })
    }
}
