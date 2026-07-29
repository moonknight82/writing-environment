#include <dlfcn.h>
#include <gtk/gtk.h>
#include <stdio.h>

#define RAINDROP_PLUGIN "/usr/lib/aarch64-linux-gnu/rpcc/librpcc_raindrop.so"

typedef void (*init_plugin_fn)(GtkWidget *parent);
typedef GtkWidget *(*get_tab_fn)(int tab);
typedef gboolean (*reboot_needed_fn)(void);
typedef void (*free_plugin_fn)(void);

static reboot_needed_fn save_display_config;

static gboolean close_window(GtkWidget *widget, GdkEvent *event, gpointer user_data)
{
    (void)widget;
    (void)event;
    (void)user_data;

    if (save_display_config != NULL) {
        save_display_config();
    }
    gtk_main_quit();
    return FALSE;
}

static void *load_symbol(void *plugin, const char *name)
{
    void *symbol = dlsym(plugin, name);
    if (symbol == NULL) {
        fprintf(stderr, "Display settings plugin is missing %s: %s\n", name, dlerror());
    }
    return symbol;
}

int main(int argc, char **argv)
{
    void *plugin;
    init_plugin_fn init_plugin;
    get_tab_fn get_tab;
    free_plugin_fn free_plugin;
    GtkWidget *window;
    GtkWidget *page;

    gtk_init(&argc, &argv);

    plugin = dlopen(RAINDROP_PLUGIN, RTLD_NOW | RTLD_LOCAL);
    if (plugin == NULL) {
        fprintf(stderr, "Could not load Raspberry Pi display settings: %s\n", dlerror());
        return 1;
    }

    init_plugin = (init_plugin_fn)load_symbol(plugin, "init_plugin");
    get_tab = (get_tab_fn)load_symbol(plugin, "get_tab");
    save_display_config = (reboot_needed_fn)load_symbol(plugin, "reboot_needed");
    free_plugin = (free_plugin_fn)load_symbol(plugin, "free_plugin");
    if (init_plugin == NULL || get_tab == NULL || save_display_config == NULL ||
        free_plugin == NULL) {
        dlclose(plugin);
        return 1;
    }

    window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "Display Settings");
    gtk_window_set_default_size(GTK_WINDOW(window), 700, 520);
    gtk_container_set_border_width(GTK_CONTAINER(window), 12);
    g_signal_connect(window, "delete-event", G_CALLBACK(close_window), NULL);

    init_plugin(window);
    page = get_tab(0);
    if (page == NULL) {
        fprintf(stderr, "Raspberry Pi display settings returned no page.\n");
        free_plugin();
        dlclose(plugin);
        return 1;
    }

    gtk_container_add(GTK_CONTAINER(window), page);
    gtk_widget_show_all(window);
    gtk_main();

    free_plugin();
    dlclose(plugin);
    return 0;
}
