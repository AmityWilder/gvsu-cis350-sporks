import time, xmlrpc.client, subprocess, atexit, tkinter as tk
from tkinter import ttk
from functions import *

IS_DEBUG_BUILD = True
if IS_DEBUG_BUILD:
    BUILD = "debug"
else:
    BUILD = "release"

# open the server in parallel
srv = subprocess.Popen([f"./target/{BUILD}/gvsu-cis350-sporks.exe"])

# create a line of communication with the server
with xmlrpc.client.ServerProxy("http://127.0.0.1:8080") as proxy:

    def close_server():
        print("attempting to close server")
        proxy.quit({})
        slept = 0
        while srv.poll() is None:
            # still running after 5 seconds
            if slept >= 2:
                print("close failed, terminating server")
                srv.terminate()
                break
            else:
                time.sleep(0.01)
                slept += 0.01
        slept = 0
        while srv.poll() is None:
            # still running 5 seconds after termination
            if slept >= 5:
                print("termination failed, killing server")
                srv.kill()
                break
            else:
                time.sleep(0.01)
                slept += 0.01
        print("finished")

    atexit.register(close_server)

    # Create the main window
    root = tk.Tk()
    #removes focus
    root.bind_all("<Button-1>", lambda event: event.widget.focus_set())
    root.title("Spork Scheduler")
    root.geometry("640x480")

    # Create a label widget
    label = tk.Label(root, text="Welcome!")
    label.pack(pady=10) # Add some vertical padding

    # Create a button widget
    button = tk.Button(root, text="Click Me", command=lambda: on_button_click(label))
    task_button = tk.Button(root, text="Add Task", command=lambda: add_task(proxy))
    user_button = tk.Button(root, text="Add User", command=lambda: add_user(proxy))
    button.pack(pady=20)
    #task_button.pack()
    #user_button.pack()
    #add manager and employee buttons
    task_visible=tk.BooleanVar(value=False)
    user_visible=tk.BooleanVar(value=False)
    manager=tk.Button(root, text="Manager", command=lambda:toggle_element(task_visible,[task_button, user_button]))
    manager.pack()
    employee=tk.Button(root, text="employee")
    employee.pack()
    

    # Define the options for the dropdown
    options = ["Apple", "Banana", "Orange", "Grape"]

    # Create a StringVar to hold the selected option
    selected_option = tk.StringVar(root)
    selected_option.set(options[0])  # Set the default value

    # Create the OptionMenu widget
    dropdown_menu = tk.OptionMenu(root, selected_option, *options)
    dropdown_menu.pack(pady=10)

    # Create a StringVar to track textbox visibility
    textbox_visible = tk.BooleanVar(value=False)

    # Create the button
    toggle_button = tk.Button(root, text="toggle Textbox", command=lambda:toggle_element(textbox_visible,entry_box))
    toggle_button.pack(pady=10)
    # Create the Entry widget (initially hidden)
    entry_box = tk.Entry(root, width=30)

    # Start the main event loop
    root.mainloop()
