import tkinter as tk

def on_button_click():
    """Function to be called when the button is clicked."""
    label.config(text="Button was clicked!")

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
button2 = tk.Button(root, text="Click It", command=on_button_click)
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