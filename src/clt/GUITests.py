import tkinter as tk
from tkinter import ttk

def on_button1_click():
    """Function to be called when the button is clicked."""
    label.config(text="Enter Manager!")

def on_button2_click():
    """Function to be called when the button is clicked."""
    label.config(text="Enter Employee!")

# Create the main window
root = tk.Tk()
root.title("Spork Scheduler")
root.geometry("500x350")

# Create a label widget
label = tk.Label(root, text="Welcome!")
label.pack(expand=True) # Add some vertical padding

#frame to hold button
button_frame=tk.Frame(root)
button_frame.pack(expand=True)
# Create a button widget
button = tk.Button(button_frame, text="Manager", command=on_button1_click)
button2 = tk.Button(button_frame, text="Employee", command=on_button2_click)
button.pack(side=tk.LEFT, padx=20)
button2.pack(side=tk.RIGHT, padx=20)






# Start the main event loop
root.mainloop()