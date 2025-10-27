import time, xmlrpc.client, subprocess, atexit, tkinter as tk
from tkinter import ttk

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

    def on_button_click():
        """Function to be called when the button is clicked."""
        label.config(text="Button was clicked!")

    def add_task():
        added = proxy.add_tasks({'to_add': [{'title': "foo"}]})

    def add_user():
        added = proxy.add_users({'to_add': [{'name': "Edward Coolguy"}]})

    # Create the main window
    root = tk.Tk()
    root.title("Simple Tkinter App")
    root.geometry("640x480")

    # Create a label widget
    label = tk.Label(root, text="Welcome!")
    label.pack(pady=10) # Add some vertical padding


    


    # Create a button widget
    button = tk.Button(root, text="Click Me", command=on_button_click)
    button2 = tk.Button(root, text="Add Task", command=add_task)
    button3 = tk.Button(root, text="Add User", command=add_user)
    button.pack(pady=20)
    button2.pack()
    button3.pack()


    


    # Start the main event loop
    root.mainloop()
