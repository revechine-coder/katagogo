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

int go_core_create(void);
int go_core_destroy(void);
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

int go_core_get_analysis(
    double* out_winrate_black, double* out_lead_black,
    int* out_move_count,
    char* out_current_player, int out_current_player_len,
    int* out_captures_black, int* out_captures_white,
    double* out_evaluation_accuracy);

int go_core_get_move_suggestions(
    MoveSuggestion* out_suggestions, int out_max, int* out_num);

int go_core_get_ownership(double* out_ownership, int out_max);

const char* go_core_last_error(void);

#ifdef __cplusplus
}
#endif

#endif
