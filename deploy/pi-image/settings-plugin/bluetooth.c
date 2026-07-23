#include <gtk/gtk.h>

static GtkWidget *control_centre;
static GtkWidget *bluetooth_page;

static void show_launch_error(const GError *error)
{
    GtkWidget *dialog = gtk_message_dialog_new(
        GTK_WINDOW(control_centre),
        GTK_DIALOG_MODAL | GTK_DIALOG_DESTROY_WITH_PARENT,
        GTK_MESSAGE_ERROR,
        GTK_BUTTONS_CLOSE,
        "Bluetooth settings could not be opened."
    );

    gtk_message_dialog_format_secondary_text(
        GTK_MESSAGE_DIALOG(dialog),
        "%s",
        error != NULL ? error->message : "Unknown error"
    );
    gtk_dialog_run(GTK_DIALOG(dialog));
    gtk_widget_destroy(dialog);
}

static void open_bluetooth_manager(GtkButton *button, gpointer user_data)
{
    GError *error = NULL;
    gchar *arguments[] = { "/usr/bin/blueman-manager", NULL };

    (void) button;
    (void) user_data;

    if (!g_spawn_async(
            NULL,
            arguments,
            NULL,
            G_SPAWN_DEFAULT,
            NULL,
            NULL,
            NULL,
            &error
        )) {
        show_launch_error(error);
        g_clear_error(&error);
    }
}

void init_plugin(GtkWidget *parent)
{
    GtkWidget *heading;
    GtkWidget *description;
    GtkWidget *note;
    GtkWidget *button;

    control_centre = parent;
    bluetooth_page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 18);
    gtk_container_set_border_width(GTK_CONTAINER(bluetooth_page), 24);

    heading = gtk_label_new(NULL);
    gtk_label_set_markup(
        GTK_LABEL(heading),
        "<span size=\"large\" weight=\"bold\">Bluetooth devices</span>"
    );
    gtk_widget_set_halign(heading, GTK_ALIGN_START);
    gtk_box_pack_start(GTK_BOX(bluetooth_page), heading, FALSE, FALSE, 0);

    description = gtk_label_new(
        "Pair, connect, trust, or remove keyboards, headphones, and other "
        "Bluetooth devices."
    );
    gtk_label_set_line_wrap(GTK_LABEL(description), TRUE);
    gtk_label_set_max_width_chars(GTK_LABEL(description), 58);
    gtk_widget_set_halign(description, GTK_ALIGN_START);
    gtk_box_pack_start(GTK_BOX(bluetooth_page), description, FALSE, FALSE, 0);

    button = gtk_button_new_with_label("Open Bluetooth Devices\342\200\246");
    gtk_widget_set_halign(button, GTK_ALIGN_START);
    gtk_widget_set_tooltip_text(
        button,
        "Open the Bluetooth device pairing and connection manager"
    );
    g_signal_connect(button, "clicked", G_CALLBACK(open_bluetooth_manager), NULL);
    gtk_box_pack_start(GTK_BOX(bluetooth_page), button, FALSE, FALSE, 0);

    note = gtk_label_new(
        "Tip: Bluetooth remains available from the panel icon beside Wi-Fi, too."
    );
    gtk_label_set_line_wrap(GTK_LABEL(note), TRUE);
    gtk_label_set_max_width_chars(GTK_LABEL(note), 58);
    gtk_widget_set_halign(note, GTK_ALIGN_START);
    gtk_style_context_add_class(
        gtk_widget_get_style_context(note),
        GTK_STYLE_CLASS_DIM_LABEL
    );
    gtk_box_pack_start(GTK_BOX(bluetooth_page), note, FALSE, FALSE, 0);

    gtk_widget_show_all(bluetooth_page);
}

int plugin_tabs(void)
{
    return 1;
}

const char *tab_name(int tab)
{
    (void) tab;
    return "Bluetooth";
}

const char *icon_name(int tab)
{
    (void) tab;
    return "bluetooth";
}

const char *tab_id(int tab)
{
    (void) tab;
    return "bluetooth";
}

GtkWidget *get_tab(int tab)
{
    (void) tab;
    return bluetooth_page;
}

gboolean reboot_needed(void)
{
    return FALSE;
}

void free_plugin(void)
{
    control_centre = NULL;
    bluetooth_page = NULL;
}
