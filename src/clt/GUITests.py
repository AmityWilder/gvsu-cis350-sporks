import time, xmlrpc.client, subprocess, atexit, tkinter as tk

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
        srv.terminate()
        time.sleep(1)
        # still running after 1 second
        if srv.poll() is None:
            srv.kill()

    atexit.register(close_server)

    def on_button_click():
        """Function to be called when the button is clicked."""
        label.config(text="Button was clicked!")

    def ping_server():
        code = 657346
        print(f"clt: pinging server... code: {code}")
        new_code = proxy.ping({'code': code})
        print(f"clt: response: {new_code}")

    # Create the main window
    root = tk.Tk()
    root.title("Simple Tkinter App")
    root.geometry("500x250")

    # Create a label widget
    label = tk.Label(root, text="Welcome!")
    label.pack(pady=10) # Add some vertical padding


    textbox = tk.Entry(root)
    textbox.pack() # Add some vertical padding
    #textline = tk.Text(root, width = 50, height = 5)
    #textline.pack() # Add some vertical padding


    # Create a button widget
    button = tk.Button(root, text="Click Me", command=on_button_click)
    button2 = tk.Button(root, text="Ping", command=ping_server)
    button.pack(pady=20)
    button2.pack()


    def get_text_data():
        data = text_widget.get("1.0", "end-1c") # "1.0" for start, "end-1c" to exclude trailing newline
        print("Text data:", type(data))
        print("Text data:", data)


    text_widget = tk.Text(root, height=5, width=30)
    text_widget.pack()

    submit_button = tk.Button(root, text="Get Text Data", command=get_text_data)
    submit_button.pack()


    # Start the main event loop
    root.mainloop()
