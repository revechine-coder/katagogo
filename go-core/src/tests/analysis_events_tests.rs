use crate::analysis_events;
use crate::analysis_parser::{CandidateAnalysis, GtpAnalysisFrame};
use crate::render_frame::CRenderFrameView;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

static CALLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
static CALLBACK_SUGGESTIONS_LEN: AtomicUsize = AtomicUsize::new(0);
static CALLBACK_FIRST_VISITS: AtomicU32 = AtomicU32::new(0);

extern "C" fn test_callback(frame: *const CRenderFrameView) {
    CALLBACK_CALLS.fetch_add(1, Ordering::SeqCst);
    if frame.is_null() {
        return;
    }

    unsafe {
        CALLBACK_SUGGESTIONS_LEN.store((*frame).suggestions_len, Ordering::SeqCst);
        if !(*frame).suggestions.is_null() && (*frame).suggestions_len > 0 {
            CALLBACK_FIRST_VISITS.store((*(*frame).suggestions).visits, Ordering::SeqCst);
        }
    }
}

#[test]
fn dispatch_analysis_frame_maps_candidates_to_render_suggestions() {
    CALLBACK_CALLS.store(0, Ordering::SeqCst);
    CALLBACK_SUGGESTIONS_LEN.store(0, Ordering::SeqCst);
    CALLBACK_FIRST_VISITS.store(0, Ordering::SeqCst);

    assert_eq!(
        analysis_events::set_analysis_callback(Some(test_callback)),
        0
    );
    analysis_events::dispatch_analysis_frame(
        GtpAnalysisFrame {
            candidates: vec![
                CandidateAnalysis {
                    coordinate: (3, 15),
                    visits: 12,
                    winrate: 0.52,
                    score_lead: 1.0,
                    pv: vec![],
                },
                CandidateAnalysis {
                    coordinate: (15, 3),
                    visits: 128,
                    winrate: 0.61,
                    score_lead: 3.5,
                    pv: vec![],
                },
            ],
        },
        19,
    );
    assert_eq!(analysis_events::clear_analysis_callback(), 0);

    assert_eq!(CALLBACK_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(CALLBACK_SUGGESTIONS_LEN.load(Ordering::SeqCst), 2);
    assert_eq!(CALLBACK_FIRST_VISITS.load(Ordering::SeqCst), 128);
}
