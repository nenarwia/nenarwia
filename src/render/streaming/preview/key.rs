use crate::render::atlas::ThumbTier;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct ThumbRequestKey {
    pub id: u64,
    pub tier: ThumbTier,
}

#[inline]
pub(crate) fn thumb_request_key(id: u64, tier: ThumbTier) -> ThumbRequestKey {
    ThumbRequestKey { id, tier }
}

#[inline]
pub(crate) fn thumb_request_id(key: ThumbRequestKey) -> u64 {
    key.id
}

#[cfg(test)]
mod tests {
    use super::{thumb_request_id, thumb_request_key};
    use crate::render::atlas::ThumbTier;

    #[test]
    fn thumb_request_key_roundtrip_id() {
        let id = 0x1_0000_0000u64 + 0x12AB_34CDu64;
        let tiers = [
            ThumbTier::Px32,
            ThumbTier::Px64,
            ThumbTier::Px128,
            ThumbTier::Px256,
            ThumbTier::Px512,
        ];
        for tier in tiers {
            let key = thumb_request_key(id, tier);
            assert_eq!(thumb_request_id(key), id);
        }
    }
}
