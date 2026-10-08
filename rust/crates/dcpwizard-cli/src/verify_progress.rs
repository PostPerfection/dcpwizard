use dcpwizard_core::verify::{VerifyProgress, VerifyStage};

const WHOLE_PERCENT: u64 = 100;

#[derive(Default)]
pub struct ProgressLines {
    last_printed: Option<(VerifyStage, u64)>,
}

impl ProgressLines {
    // the GUI parses these lines off stderr, one per whole percent
    pub fn line_for(&mut self, report: VerifyProgress) -> Option<String> {
        let percent = report
            .done
            .saturating_mul(WHOLE_PERCENT)
            .checked_div(report.total)
            .unwrap_or(WHOLE_PERCENT);
        let printed = Some((report.stage, percent));
        if self.last_printed == printed {
            return None;
        }
        self.last_printed = printed;
        Some(format!(
            "progress {} {} {}",
            stage_name(report.stage),
            report.done,
            report.total
        ))
    }
}

fn stage_name(stage: VerifyStage) -> &'static str {
    match stage {
        VerifyStage::HashCheck => "hashes",
        VerifyStage::FrameScan => "frames",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASHED_BYTES: u64 = 20_000_000_000;
    const READ_CHUNK_BYTES: u64 = 1 << 20;
    const MOST_LINES_PER_STAGE: usize = 101;

    fn lines_for(lines: &mut ProgressLines, reports: &[VerifyProgress]) -> Vec<String> {
        reports
            .iter()
            .filter_map(|report| lines.line_for(*report))
            .collect()
    }

    #[test]
    fn a_long_hash_check_prints_one_line_per_whole_percent() {
        let mut reports = vec![VerifyProgress {
            stage: VerifyStage::HashCheck,
            done: 0,
            total: HASHED_BYTES,
        }];
        let mut done = 0;
        while done < HASHED_BYTES {
            done = (done + READ_CHUNK_BYTES).min(HASHED_BYTES);
            reports.push(VerifyProgress {
                stage: VerifyStage::HashCheck,
                done,
                total: HASHED_BYTES,
            });
        }

        let lines = lines_for(&mut ProgressLines::default(), &reports);

        assert_eq!(lines.len(), MOST_LINES_PER_STAGE);
        assert_eq!(lines[0], format!("progress hashes 0 {HASHED_BYTES}"));
        assert_eq!(
            lines.last().unwrap(),
            &format!("progress hashes {HASHED_BYTES} {HASHED_BYTES}")
        );
    }

    #[test]
    fn the_next_reel_starts_its_frame_scan_on_a_line_of_its_own() {
        let frames = |done| VerifyProgress {
            stage: VerifyStage::FrameScan,
            done,
            total: 1,
        };

        let lines = lines_for(
            &mut ProgressLines::default(),
            &[frames(0), frames(1), frames(0), frames(1)],
        );

        assert_eq!(
            lines,
            [
                "progress frames 0 1",
                "progress frames 1 1",
                "progress frames 0 1",
                "progress frames 1 1"
            ]
        );
    }
}
