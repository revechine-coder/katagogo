#ifndef GO_CORE_H
#define GO_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint8_t col;
    uint8_t row;
    uint8_t color;
} StoneRender;

typedef struct {
    uint8_t col;
    uint8_t row;
    uint32_t move_number;
    uint8_t is_last;
} MoveLabel;

typedef struct {
    uint8_t col;
    uint8_t row;
    double winrate;
    double lead;
    uint32_t visits;
    uint32_t order;
} MoveSuggestion;

typedef struct {
    uint32_t move_number;
    uint8_t color;          // 0 = black, 1 = white
    uint8_t quality;        // 0 = good, 1 = bad_move, 2 = slack_move
    uint8_t _pad1[2];
    uint8_t vertex[8];      // null-terminated GTP vertex
    double winrate_before;
    double winrate_after;
    double score_before;
    double score_after;
    double winrate_drop;
    double score_drop;
    uint8_t ai_best_col;
    uint8_t ai_best_row;
    uint8_t has_ai_best;    // 0 or 1
    uint8_t suggestions_count;
    uint8_t _pad2[4];
} CReviewedMove;

typedef struct {
    uint8_t col;
    uint8_t row;
} StarPoint;

typedef struct {
    uint8_t board_size;
    uint8_t current_player;
    uint8_t _padding[2];
    uint32_t move_count;
    uint32_t captures_black;
    uint32_t captures_white;
    int16_t last_move_col;
    int16_t last_move_row;
    const StoneRender* stones;
    uintptr_t stones_len;
    const MoveLabel* move_labels;
    uintptr_t move_labels_len;
    const StarPoint* star_points;
    uintptr_t star_points_len;
    const MoveSuggestion* suggestions;
    uintptr_t suggestions_len;
} CRenderFrameView;

typedef void (*FfiAnalysisCallback)(const CRenderFrameView* frame);

int go_core_create(void);
int go_core_destroy(void);
int go_core_set_analysis_callback(FfiAnalysisCallback callback);
int go_core_clear_analysis_callback(void);
int go_core_emit_current_analysis_frame(void);
int go_core_start(const char* binary_path, const char* config_path,
                  const char* model_path, int board_size, double timeout_secs);
int go_core_close(void);

int go_core_play(const char* color, const char* vertex);
int go_core_genmove(const char* color,
                    char* out_vertex, int out_vertex_len,
                    double* out_winrate, double* out_lead);
int go_core_undo(void);
int go_core_reset(void);
int go_core_final_score(char* out_score, int out_score_len);

int go_core_set_level(int level);
int go_core_set_handicap(int count);
int go_core_set_suggestions_enabled(int enabled);
int go_core_refresh_move_suggestions(void);

int go_core_get_render_frame(
    StoneRender* out_stones, int out_max_stones, int* out_num_stones,
    MoveLabel* out_move_labels, int out_max_labels, int* out_num_labels,
    int* out_board_size,
    int* out_last_move_col, int* out_last_move_row,
    int* out_move_count,
    char* out_current_player, int out_current_player_len);

const CRenderFrameView* go_core_get_render_frame_view(void);

int go_core_get_play_tree_cursor(
    uint32_t* out_path, int out_max_path, int* out_path_len,
    uint32_t* out_current_move_number,
    int* out_child_count,
    int* out_active_line_len);

int go_core_get_analysis(
    double* out_winrate_black, double* out_lead_black,
    int* out_move_count,
    char* out_current_player, int out_current_player_len,
    int* out_captures_black, int* out_captures_white,
    double* out_evaluation_accuracy);

int go_core_get_move_suggestions(
    MoveSuggestion* out_suggestions, int out_max, int* out_num);

int go_core_get_ownership(double* out_ownership, int out_max);

int go_core_run_auto_review(int num_visits);
int go_core_get_auto_review_moves(CReviewedMove* out_moves, int out_max, int* out_num);

const char* go_core_last_error(void);

#ifdef __cplusplus
}
#endif

#endif
