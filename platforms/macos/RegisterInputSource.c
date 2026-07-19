#include <Carbon/Carbon.h>
#include <CoreFoundation/CoreFoundation.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static bool boolean_property(TISInputSourceRef source, CFStringRef key) {
  CFTypeRef value = TISGetInputSourceProperty(source, key);
  return value != NULL && CFGetTypeID(value) == CFBooleanGetTypeID() &&
         CFBooleanGetValue((CFBooleanRef)value);
}

static void print_source_id(TISInputSourceRef source) {
  CFStringRef source_id =
      TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
  char buffer[512] = "unknown";
  if (source_id != NULL) {
    CFStringGetCString(source_id, buffer, sizeof(buffer), kCFStringEncodingUTF8);
  }
  printf("%s", buffer);
}

static void print_string_property(TISInputSourceRef source, CFStringRef key,
                                  const char *label) {
  CFTypeRef value = TISGetInputSourceProperty(source, key);
  char buffer[512] = "(none)";
  if (value != NULL && CFGetTypeID(value) == CFStringGetTypeID()) {
    CFStringGetCString((CFStringRef)value, buffer, sizeof(buffer),
                       kCFStringEncodingUTF8);
  }
  printf("  %s: %s\n", label, buffer);
}

static void print_source(TISInputSourceRef source) {
  print_string_property(source, kTISPropertyInputSourceID, "source id");
  print_string_property(source, kTISPropertyBundleID, "bundle id");
  print_string_property(source, kTISPropertyInputModeID, "mode id");
  print_string_property(source, kTISPropertyLocalizedName, "name");
  print_string_property(source, kTISPropertyInputSourceCategory, "category");
  print_string_property(source, kTISPropertyInputSourceType, "type");
  printf("  enabled: %s\n",
         boolean_property(source, kTISPropertyInputSourceIsEnabled) ? "yes"
                                                                    : "no");
  printf("  enable capable: %s\n",
         boolean_property(source, kTISPropertyInputSourceIsEnableCapable)
             ? "yes"
             : "no");
  printf("  select capable: %s\n",
         boolean_property(source, kTISPropertyInputSourceIsSelectCapable)
             ? "yes"
             : "no");
}

int main(int argc, const char *argv[]) {
  bool print_current = argc == 2 && strcmp(argv[1], "--current") == 0;
  bool diagnose_all = argc == 2 && strcmp(argv[1], "--diagnose-all") == 0;
  bool register_bundle = argc == 4 && strcmp(argv[3], "--register") == 0;
  bool select_bundle = argc == 4 && strcmp(argv[3], "--select") == 0;
  bool diagnose_bundle = argc == 4 && strcmp(argv[3], "--diagnose") == 0;
  bool select_id = argc == 5 && strcmp(argv[3], "--select-id") == 0;
  if (!print_current && !diagnose_all && !register_bundle && !select_bundle &&
      !diagnose_bundle && !select_id) {
    fprintf(stderr,
            "usage: register-input-source --current|--diagnose-all | BUNDLE_PATH "
            "BUNDLE_ID --register|--select|--diagnose|--select-id "
            "INPUT_SOURCE_ID\n");
    return 64;
  }

  if (print_current) {
    TISInputSourceRef current = TISCopyCurrentKeyboardInputSource();
    if (current == NULL) {
      fprintf(stderr, "TISCopyCurrentKeyboardInputSource returned NULL\n");
      return 1;
    }
    print_source(current);
    CFRelease(current);
    return 0;
  }

  if (diagnose_all) {
    CFArrayRef all_sources = TISCreateInputSourceList(NULL, true);
    if (all_sources == NULL) {
      fprintf(stderr, "TISCreateInputSourceList returned NULL\n");
      return 1;
    }
    CFIndex all_count = CFArrayGetCount(all_sources);
    printf("Found %ld input source(s)\n", (long)all_count);
    for (CFIndex index = 0; index < all_count; index++) {
      printf("source %ld:\n", (long)(index + 1));
      print_source(
          (TISInputSourceRef)CFArrayGetValueAtIndex(all_sources, index));
    }
    CFRelease(all_sources);
    return 0;
  }

  if (register_bundle) {
    CFStringRef path = CFStringCreateWithCString(
        kCFAllocatorDefault, argv[1], kCFStringEncodingUTF8);
    CFURLRef url = CFURLCreateWithFileSystemPath(
        kCFAllocatorDefault, path, kCFURLPOSIXPathStyle, true);
    OSStatus register_status = TISRegisterInputSource(url);
    CFRelease(url);
    CFRelease(path);
    if (register_status != noErr) {
      fprintf(stderr, "TISRegisterInputSource failed: %d\n", register_status);
      return 1;
    }
    printf("Registered input method bundle\n");
    return 0;
  }

  const char *filter_value = select_id ? argv[4] : argv[2];
  CFStringRef filter = CFStringCreateWithCString(
      kCFAllocatorDefault, filter_value, kCFStringEncodingUTF8);
  const void *keys[] = {
      select_id ? kTISPropertyInputSourceID : kTISPropertyBundleID};
  const void *values[] = {filter};
  CFDictionaryRef properties = CFDictionaryCreate(
      kCFAllocatorDefault, keys, values, 1, &kCFTypeDictionaryKeyCallBacks,
      &kCFTypeDictionaryValueCallBacks);
  CFArrayRef sources = TISCreateInputSourceList(properties, true);
  CFRelease(properties);
  CFRelease(filter);

  if (sources == NULL) {
    fprintf(stderr, "TISCreateInputSourceList returned NULL\n");
    return 1;
  }

  CFIndex count = CFArrayGetCount(sources);
  if (count == 0) {
    fprintf(stderr, "registered bundle but found no input sources\n");
    CFRelease(sources);
    return 1;
  }

  if (diagnose_bundle) {
    printf("Found %ld input source(s)\n", (long)count);
    for (CFIndex index = 0; index < count; index++) {
      printf("source %ld:\n", (long)(index + 1));
      print_source(
          (TISInputSourceRef)CFArrayGetValueAtIndex(sources, index));
    }
    CFRelease(sources);
    return 0;
  }

  bool enabled_source = false;
  for (CFIndex index = 0; index < count; index++) {
    TISInputSourceRef source =
        (TISInputSourceRef)CFArrayGetValueAtIndex(sources, index);
    if (boolean_property(source, kTISPropertyInputSourceIsEnableCapable) &&
        !boolean_property(source, kTISPropertyInputSourceIsEnabled)) {
      OSStatus status = TISEnableInputSource(source);
      if (status != noErr) {
        fprintf(stderr, "TISEnableInputSource failed: %d\n", status);
        CFRelease(sources);
        return 1;
      }
      enabled_source = true;
    }
  }

  if (enabled_source) {
    sleep(2);
  }

  bool selected = false;
  for (CFIndex index = 0; index < count; index++) {
    TISInputSourceRef source =
        (TISInputSourceRef)CFArrayGetValueAtIndex(sources, index);
    if (!boolean_property(source, kTISPropertyInputSourceIsSelectCapable)) {
      continue;
    }
    OSStatus status = TISSelectInputSource(source);
    if (status == noErr) {
      printf("Selected input source: ");
      print_source_id(source);
      printf("\n");
      selected = true;
      break;
    }
    fprintf(stderr, "TISSelectInputSource failed for ");
    print_source_id(source);
    fprintf(stderr, ": %d\n", status);
  }

  CFRelease(sources);
  if (!selected) {
    fprintf(stderr, "input source was registered but could not be selected\n");
    return 2;
  }
  return 0;
}
