use crate::mxf_wrap::AudioInputOrder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelPlacer {
    AudioMap,
    InputOrder,
    Upmix,
    ChannelSet,
}

// the input order describes a plain six-channel WAV, which the upmix and a channel set replace
const CONFLICTING_PLACERS: [(ChannelPlacer, ChannelPlacer); 4] = [
    (ChannelPlacer::AudioMap, ChannelPlacer::InputOrder),
    (ChannelPlacer::AudioMap, ChannelPlacer::Upmix),
    (ChannelPlacer::InputOrder, ChannelPlacer::Upmix),
    (ChannelPlacer::InputOrder, ChannelPlacer::ChannelSet),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct ChannelPlacement {
    pub audio_map: bool,
    pub input_order: AudioInputOrder,
    pub upmix: bool,
    pub channel_set: bool,
}

impl ChannelPlacement {
    fn uses(&self, placer: ChannelPlacer) -> bool {
        match placer {
            ChannelPlacer::AudioMap => self.audio_map,
            ChannelPlacer::InputOrder => self.input_order == AudioInputOrder::LrcLsRsLfe,
            ChannelPlacer::Upmix => self.upmix,
            ChannelPlacer::ChannelSet => self.channel_set,
        }
    }

    pub fn conflict(&self) -> Option<(ChannelPlacer, ChannelPlacer)> {
        CONFLICTING_PLACERS
            .into_iter()
            .find(|(first, second)| self.uses(*first) && self.uses(*second))
    }

    // the pair `placer` would form once set, whether or not it is set now
    pub fn conflict_if_set(&self, placer: ChannelPlacer) -> Option<(ChannelPlacer, ChannelPlacer)> {
        CONFLICTING_PLACERS.into_iter().find(|(first, second)| {
            (*first == placer && self.uses(*second)) || (*second == placer && self.uses(*first))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LRC: AudioInputOrder = AudioInputOrder::LrcLsRsLfe;

    #[test]
    fn the_input_order_conflicts_with_the_upmix_a_channel_set_and_a_map() {
        for (placement, expected) in [
            (
                ChannelPlacement {
                    input_order: LRC,
                    upmix: true,
                    ..Default::default()
                },
                (ChannelPlacer::InputOrder, ChannelPlacer::Upmix),
            ),
            (
                ChannelPlacement {
                    input_order: LRC,
                    channel_set: true,
                    ..Default::default()
                },
                (ChannelPlacer::InputOrder, ChannelPlacer::ChannelSet),
            ),
            (
                ChannelPlacement {
                    input_order: LRC,
                    audio_map: true,
                    ..Default::default()
                },
                (ChannelPlacer::AudioMap, ChannelPlacer::InputOrder),
            ),
            (
                ChannelPlacement {
                    upmix: true,
                    audio_map: true,
                    ..Default::default()
                },
                (ChannelPlacer::AudioMap, ChannelPlacer::Upmix),
            ),
        ] {
            assert_eq!(placement.conflict(), Some(expected), "{placement:?}");
        }
    }

    #[test]
    fn the_dcp_order_beside_the_upmix_or_a_channel_set_is_no_conflict() {
        let placement = ChannelPlacement {
            upmix: true,
            channel_set: true,
            ..Default::default()
        };
        assert_eq!(placement.conflict(), None);
        assert_eq!(
            placement.conflict_if_set(ChannelPlacer::InputOrder),
            Some((ChannelPlacer::InputOrder, ChannelPlacer::Upmix))
        );
        assert_eq!(
            ChannelPlacement::default().conflict_if_set(ChannelPlacer::InputOrder),
            None
        );
    }
}
