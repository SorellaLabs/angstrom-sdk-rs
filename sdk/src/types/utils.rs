use alloy_eips::BlockNumberOrTag;
use angstrom_types_primitives::ANGSTROM_DEPLOYED_BLOCK;

pub(crate) fn chunk_blocks(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<(BlockNumberOrTag, BlockNumberOrTag)> {
    let mut start_block = start_block.unwrap_or_else(|| *ANGSTROM_DEPLOYED_BLOCK.get().unwrap());
    if let Some(eb) = end_block {
        let mut tags = Vec::new();
        while eb - start_block > 1000 {
            tags.push((start_block.into(), (start_block + 1000).into()));
            start_block += 1000;
        }
        tags.push((start_block.into(), eb.into()));
        tags
    } else {
        vec![(start_block.into(), BlockNumberOrTag::Latest)]
    }
}
