#ifndef IME_FFI_H
#define IME_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ImeHandle ImeHandle;

typedef struct ImeBuffer {
  uint8_t *data;
  size_t len;
  size_t capacity;
} ImeBuffer;

enum ImeEventKind {
  IME_EVENT_CHARACTER = 0,
  IME_EVENT_SPACE = 1,
  IME_EVENT_ENTER = 2,
  IME_EVENT_ESCAPE = 3,
  IME_EVENT_BACKSPACE = 4,
  IME_EVENT_NEXT_CANDIDATE = 5,
  IME_EVENT_PREVIOUS_CANDIDATE = 6,
  IME_EVENT_SELECT_CANDIDATE = 7,
  IME_EVENT_ACCEPT_CANDIDATE = 8,
};

ImeHandle *ime_create(void);
ImeHandle *ime_create_with_data_dir(const uint8_t *data_dir,
                                    size_t data_dir_len);
void ime_destroy(ImeHandle *handle);
ImeBuffer ime_process(ImeHandle *handle, uint32_t event_kind, uint32_t value);
ImeBuffer ime_set_options(ImeHandle *handle, bool live_conversion,
                          bool history_completion);
ImeBuffer ime_set_options_v2(ImeHandle *handle, bool live_conversion,
                             bool history_completion,
                             uint32_t dictionary_packs);
ImeBuffer ime_set_options_v3(ImeHandle *handle, bool live_conversion,
                             bool history_completion, bool history_learning,
                             uint32_t dictionary_packs);
ImeBuffer ime_reload_user_data(ImeHandle *handle);
void ime_buffer_destroy(ImeBuffer buffer);

#ifdef __cplusplus
}
#endif

#endif
