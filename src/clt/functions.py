import time, xmlrpc.client, subprocess, atexit, tkinter as tk

def on_button_click(label):
    """Function to be called when the button is clicked."""
    label.config(text="Button was clicked!")

def add_task(proxy):
    added = proxy.add_tasks({'to_add': [{'title': "foo"}]})

def add_user(proxy):
    added = proxy.add_users({'to_add': [{'name': "Edward Coolguy"}]})

def toggle_elements(curr_element_visible,opp_element_visible, element, opp_element, Cancel):
    # hide elements
    if curr_element_visible.get():  # If textbox is currently visible
        for i in range(len(element)):
            current=element[i]
            current.pack_forget()  # Hide the textbox
        curr_element_visible.set(False)
        Cancel.pack_forget()
    
    # show elements
    else:  # If textbox is currently hidden
        if opp_element_visible.get():
            for n in range(len(opp_element)):
                current=opp_element[n]
                current.pack_forget()  # Hide the textbox
            opp_element_visible.set(False)
        for j in range(len(element)):
            current=element[j]
            current.pack(pady=5)  # Show the textbox
        curr_element_visible.set(True)
        Cancel.pack(side=tk.TOP)
        