#ifndef IME_FFI_H
#define IME_FFI_H

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
};

ImeHandle *ime_create(void);
void ime_destroy(ImeHandle *handle);
ImeBuffer ime_process(ImeHandle *handle, uint32_t event_kind, uint32_t scalar);
void ime_buffer_destroy(ImeBuffer buffer);

#ifdef __cplusplus
}
#endif

#endif

