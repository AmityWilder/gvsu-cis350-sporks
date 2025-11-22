def on_button_click(label):
    """Function to be called when the button is clicked."""
    label.config(text="Button was clicked!")

def add_task(proxy):
    added = proxy.add_tasks({'to_add': [{'title': "foo"}]})

def add_user(proxy):
    added = proxy.add_users({'to_add': [{'name': "Edward Coolguy"}]})

def toggle_textbox(textbox_visible,entry_box,toggle_button):
    if textbox_visible.get():  # If textbox is currently visible
        entry_box.pack_forget()  # Hide the textbox
        textbox_visible.set(False)
        toggle_button.config(text="Show Textbox")
    else:  # If textbox is currently hidden
        entry_box.pack()  # Show the textbox
        textbox_visible.set(True)
        toggle_button.config(text="Hide Textbox")