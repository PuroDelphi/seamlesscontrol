/*
 * Read-only Wayland capability probe for SeamlessControl.
 * Build: cc -Wall -Wextra -Werror -std=c11 wayland_registry.c -o wayland_registry $(pkg-config --cflags --libs wayland-client)
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wayland-client.h>

struct probe_state {
    unsigned int globals;
    unsigned int capture;
    unsigned int virtual_pointer;
    unsigned int virtual_keyboard;
    unsigned int data_control;
};

static void on_global(void *data, struct wl_registry *registry, uint32_t name,
                      const char *interface, uint32_t version) {
    (void)registry;
    struct probe_state *state = data;
    state->globals++;
    printf("%-43s v%-2u id=%u\n", interface, version, name);
    if (strcmp(interface, "hyprland_input_capture_manager_v1") == 0)
        state->capture++;
    if (strcmp(interface, "zwlr_virtual_pointer_manager_v1") == 0)
        state->virtual_pointer++;
    if (strcmp(interface, "zwp_virtual_keyboard_manager_v1") == 0)
        state->virtual_keyboard++;
    if (strcmp(interface, "ext_data_control_manager_v1") == 0 ||
        strcmp(interface, "zwlr_data_control_manager_v1") == 0)
        state->data_control++;
}

static void on_global_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data;
    (void)registry;
    (void)name;
}

static const struct wl_registry_listener listener = {
    .global = on_global,
    .global_remove = on_global_remove,
};

int main(void) {
    struct wl_display *display = wl_display_connect(NULL);
    if (!display) {
        fprintf(stderr, "No se pudo conectar al compositor Wayland. Ejecute el probe dentro de una sesión Omarchy.\n");
        return EXIT_FAILURE;
    }

    struct wl_registry *registry = wl_display_get_registry(display);
    if (!registry) {
        fprintf(stderr, "No se pudo leer el registro Wayland.\n");
        wl_display_disconnect(display);
        return EXIT_FAILURE;
    }

    struct probe_state state = {0};
    wl_registry_add_listener(registry, &listener, &state);
    if (wl_display_roundtrip(display) < 0) {
        fprintf(stderr, "Falló la lectura de capacidades Wayland.\n");
        wl_registry_destroy(registry);
        wl_display_disconnect(display);
        return EXIT_FAILURE;
    }

    printf("\nResumen: %u interfaces; capture=%s; pointer=%s; keyboard=%s; clipboard=%s\n",
           state.globals, state.capture ? "sí" : "no",
           state.virtual_pointer ? "sí" : "no",
           state.virtual_keyboard ? "sí" : "no",
           state.data_control ? "sí" : "no");
    wl_registry_destroy(registry);
    wl_display_disconnect(display);
    return EXIT_SUCCESS;
}
