import time, xmlrpc.client, subprocess, atexit, tkinter as tk
from tkinter import ttk
from functions import *




# take the data
lst = [('ID','Name','Location','Age'),
       (1,'Raj','Mumbai', 19),
       (2,'Aaryan','Pune',18),
       (3,'Vaishnavi','Mumbai',20),
       (4,'Rachna','Mumbai',21),
       (5,'Shubham','Delhi',21)]
 
# find total number of rows and
# columns in list
total_rows = len(lst)
total_columns = len(lst[0])

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

    start_center=tk.Frame(root)
    center=tk.Frame(root)
    below_center=tk.Frame(root)
    
    

    # Create a label widget
    label = tk.Label(root, text="Welcome!")
    label.pack(pady=10) # Add some vertical padding

    # Create a button widget
    Cancel_button = tk.Button(root, text="Cancel")
    
    #manager elements
    task_button = tk.Button(center, text="Add Task", command=lambda: add_task(proxy, task_name))
    task_name = tk.Entry(center, width=30)
    title=task_name.get() # stores string entered into the text box
    #desc
    #skills
    #deadline
    #deps
    user_button = tk.Button(center, text="Add Employee", command=lambda: add_user(proxy,user_box))
    user_box = tk.Entry(center, width=30)
    user=user_box.get() # stores string entered into the text box
    timeslot_button = tk.Button(center, text="Add Timeslot")

    managerlist=[task_button, task_name,user_button, user_box, timeslot_button]


    # employee elemenets
    employee_time_button = tk.Button(center, text="Add Time")
    
    employee_skills_button = tk.Button(center, text="Add Skill")

    orderframe=tk.Frame(center)
    # Define the options for the dropdown
    names = ['-']

    names_label = tk.Label(orderframe, text="Select Employee:")
    # Create a StringVar to hold the selected option
    selected_name = tk.StringVar(orderframe)
    selected_name.set(names[0])  # Set the default value

    # Create the OptionMenu widget
    names_menu = tk.OptionMenu(orderframe, selected_name, *names)
    names_label.pack(side=tk.LEFT, padx=5)
    names_menu.pack(side=tk.LEFT)

    frame_toggled=tk.BooleanVar(value=False)
    table_button = tk.Button(center, text='create table', command=lambda: form_table(frame_toggled,below_center,lst))

    # toggled elements
    
    employeelist=[orderframe, employee_time_button, employee_skills_button,table_button]


    #add manager and employee buttons
    manager_toggled=tk.BooleanVar(value=False)
    employee_toggled=tk.BooleanVar(value=False)
    edit_toggled=tk.BooleanVar(value=False)

    toggledlist=[manager_toggled,employee_toggled,edit_toggled]

    

    start_center.pack(pady= 10)
    
    manager=tk.Button(start_center, text="Manager", command=lambda:toggle_elements(manager_toggled,employee_toggled,managerlist,employeelist, Cancel_button))
    manager.pack(side=tk.LEFT, padx= 5)

    employee=tk.Button(start_center, text="Employee", command=lambda:toggle_elements(employee_toggled,manager_toggled,employeelist,managerlist, Cancel_button))
    employee.pack(side=tk.LEFT)

    edit_button=tk.Button(start_center, text="Edit")
    edit_button.pack(padx=5)

    center.pack(anchor='center')
    
    
    
    editlist=[] #figure out what edit needs



    Cancel_button.config(command=lambda: cancel(toggledlist,managerlist,employeelist, Cancel_button))

    

    # Create the button
    
    # Create the Entry widget (initially hidden)
    entry_box = tk.Entry(root, width=30)
    entered=entry_box.get() # stores string entered into the text box

    # Start the main event loop
    root.mainloop()
