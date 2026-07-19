#include "ime_ffi.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
  ImeHandle *handle = ime_create();
  assert(handle != NULL);

  const char *input = "nihon";
  for (size_t index = 0; input[index] != '\0'; ++index) {
    ImeBuffer response =
        ime_process(handle, IME_EVENT_CHARACTER, (uint32_t)input[index]);
    assert(response.data != NULL);
    ime_buffer_destroy(response);
  }

  ImeBuffer response = ime_process(handle, IME_EVENT_SPACE, 0);
  assert(response.data != NULL);

  char json[1024];
  assert(response.len < sizeof(json));
  memcpy(json, response.data, response.len);
  json[response.len] = '\0';
  assert(strstr(json, "show_candidates") != NULL);

  ime_buffer_destroy(response);
  ime_destroy(handle);
  puts("C ABI smoke test passed");
  return 0;
}

