import time, xmlrpc.client, subprocess, atexit, tkinter as tk

def on_button_click(label):
    """Function to be called when the button is clicked."""
    label.config(text="Button was clicked!")

def add_task(proxy):
    added = proxy.add_tasks({'to_add': [{'title': "foo"}]})

def add_user(proxy):
    added = proxy.add_users({'to_add': [{'name': "Edward Coolguy"}]})

def toggle_elements(element_visible,element):
    # hide elements
    if element_visible.get():  # If textbox is currently visible
        for i in range(len(element)):
            current=element[i]
            current.pack_forget()  # Hide the textbox
        element_visible.set(False)
    
    # show elements
    else:  # If textbox is currently hidden
        for j in range(len(element)):
            current=element[j]
            current.pack(side=tk.LEFT,padx=5)  # Show the textbox
        element_visible.set(True)
        