import time, xmlrpc.client, subprocess, atexit, tkinter as tk
from tkinter import ttk
from functions import *




# take the data


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
    #root.bind_all("<Button-1>", lambda event: event.widget.focus_set())
    root.title("Spork Scheduler")
    root.geometry("1100x480")
    


    tabControl = ttk.Notebook(root)

    # tabs
    tab1 = ttk.Frame(tabControl)
    tab2 = ttk.Frame(tabControl)
    tab3 = ttk.Frame(tabControl)
    tab4 = ttk.Frame(tabControl)
    
    
    

    tabControl.add(tab1, text ='Shifts')
    tabControl.add(tab2, text ='Employees')
    tabControl.add(tab3, text ='Tasks')
    tabControl.add(tab4, text ='Schedule')

    tabControl.pack(expand = 1, fill ="both")
    
    

    # frames
    shift_center=ttk.Frame(tab1)
    shift_table=ttk.Frame(tab1)
    shiftcanvas=tk.Canvas(shift_table)
    sfttable=ttk.Frame(shiftcanvas)
    shiftbar=tk.Scrollbar(shift_table)
    shiftcanvas.config(yscrollcommand=shiftbar.set)
    shiftbar.config(orient=tk.VERTICAL, command=shiftcanvas.yview)
    shiftbar.pack(fill=tk.Y,side=tk.RIGHT,expand=tk.FALSE)
    shiftcanvas.pack(fill=tk.BOTH, side=tk.LEFT, expand=tk.TRUE)
    shiftcanvas.create_window(0,0, window=sfttable,anchor=tk.NW)
    sft_bottom=ttk.Frame(tab1)

    employee_center=ttk.Frame(tab2)
    employee_table=ttk.Frame(tab2)
    empcanvas=tk.Canvas(employee_table)
    emptable=ttk.Frame(empcanvas)
    empbar=tk.Scrollbar(employee_table)
    empcanvas.config(yscrollcommand=empbar.set)
    empbar.config(orient=tk.VERTICAL, command=empcanvas.yview)
    empbar.pack(fill=tk.Y,side=tk.RIGHT,expand=tk.FALSE)
    empcanvas.pack(fill=tk.BOTH, side=tk.LEFT, expand=tk.TRUE)
    empcanvas.create_window(0,0, window=emptable,anchor=tk.NW)
    emp_bottom=ttk.Frame(tab2)

    task_center=ttk.Frame(tab3)
    task_table=ttk.Frame(tab3)
    tskcanvas=tk.Canvas(task_table)
    tsktable=ttk.Frame(tskcanvas)
    tskbar=tk.Scrollbar(task_table)
    tskcanvas.config(yscrollcommand=tskbar.set)
    tskbar.config(orient=tk.VERTICAL, command=tskcanvas.yview)
    tskbar.pack(fill=tk.Y,side=tk.RIGHT,expand=tk.FALSE)
    tskcanvas.pack(fill=tk.BOTH, side=tk.LEFT, expand=tk.TRUE)
    tskcanvas.create_window(0,0, window=tsktable,anchor=tk.NW)
    tsk_bottom=ttk.Frame(tab3)

    schedule_center=ttk.Frame(tab4)
    schedule_image=ttk.Frame(tab4)

    

    # shift tab
    sft_lst = ['Name','Start','End','Min Employees']
    #list of text boxes used to get 
    sft_boxes=[[],[]]

    # columns in list
    sft_columns = len(sft_lst[0])
    ttk.Label(tab1,text='Create shifts',font=('Arial',14,'bold')).pack(pady=10)

    shifttab=Table(sfttable,shiftcanvas,shift_center,sft_lst)
    sftsave=ttk.Button(sft_bottom,text='Save',command=lambda:saving(proxy,shifttab,'shift'))
    sftsave.pack(side=tk.BOTTOM)

    shift_center.pack()
    shift_table.pack(pady=20, padx=75, fill='x')
    sft_bottom.pack()


    # employee tab
    emp_lst = ['Name']
    emp_boxes=[]

    # columns in list
    emp_columns = len(emp_lst[0])

    ttk.Label(tab2,text='Employees',font=('Arial',14,'bold')).pack(pady=10)
    #employeelist=['-',
    #              'example',
    #              'jim']
    #selected_name = tk.StringVar()
    #namemenue = ttk.Combobox(employee_center, width=30,values=employeelist,textvariable=selected_name)
    #namemenue.grid(row=1,column=0)
    employeetab=Table(emptable,empcanvas,employee_center,emp_lst)
    empsave=ttk.Button(emp_bottom,text='Save',command=lambda:saving(proxy,employeetab,'user'))
    empsave.pack(side=tk.BOTTOM)

    employee_center.pack()
    employee_table.pack(pady=20, padx=25, fill='x')
    emp_bottom.pack()


    # task tab
    tsk_lst = ['Name','Deadline','Description']
    tsk_boxes=[[],[]]

    # columns in list
    tsk_columns = len(tsk_lst[0])
    ttk.Label(tab3,text='Create Tasks',font=('Arial',14,'bold')).pack(pady=10)
    tasktab=Table(tsktable,tskcanvas,task_center,tsk_lst)
    tsksave=ttk.Button(tsk_bottom,text='Save',command=lambda:saving(proxy,tasktab,'task'))
    tsksave.pack(side=tk.BOTTOM)

    task_button = tk.Button(tsk_bottom, text="Add Task", command=lambda: add_task(proxy, [task_name,task_desc,task_dead]))
    task_name = tk.Entry(tsk_bottom, width=30)
    task_desc = tk.Entry(tsk_bottom, width=30)
    task_dead = tk.Entry(tsk_bottom, width=30)
    task_button.pack()
    task_name.pack()
    task_desc.pack()
    task_dead.pack()
    
    

    task_center.pack()
    task_table.pack(pady=20, padx=25, fill='x')
    tsk_bottom.pack()
    
    
    # schedule tab
    ttk.Label(tab4,text='Create Schedule',font=('Arial',14,'bold')).pack(pady=10)

    create_schedule=ttk.Button(schedule_center,text='Create schedule')
    create_schedule.pack(side=tk.LEFT)
    # load previous information
    load_data=ttk.Button(schedule_center,text='Load')
    load_data.pack(side=tk.LEFT)

    schedule_center.pack()

    # frame that displays schedule
    schedule_image.pack()



    # center=tk.Frame(root)
    # below_center=tk.Frame(root)
    
    

    # # Create a label widget
    # label = tk.Label(root, text="Welcome!")
    # label.pack(pady=10) # Add some vertical padding

    # # Create a button widget
    # Cancel_button = tk.Button(root, text="Cancel")
    
    # #manager elements
    # task_button = tk.Button(center, text="Add Task", command=lambda: add_task(proxy, task_name))
    # task_name = tk.Entry(center, width=30)
    # title=task_name.get() # stores string entered into the text box
    # #desc
    # #skills
    # #deadline
    # #deps
    # user_button = tk.Button(center, text="Add Employee", command=lambda: add_user(proxy,user_box))
    # user_box = tk.Entry(center, width=30)
    # user=user_box.get() # stores string entered into the text box
    # timeslot_button = tk.Button(center, text="Add Timeslot")

    # managerlist=[task_button, task_name,user_button, user_box, timeslot_button]


    # # employee elemenets
    # employee_time_button = tk.Button(center, text="Add Time")
    
    # employee_skills_button = tk.Button(center, text="Add Skill")

    # orderframe=tk.Frame(center)
    # # Define the options for the dropdown
    

    # names_label = tk.Label(orderframe, text="Select Employee:")
    # # Create a StringVar to hold the selected option
    # selected_name = tk.StringVar()
    

    # # Create the OptionMenu widget
    # names_menu = ttk.Combobox(orderframe, width=30, textvariable=selected_name)
    # names_menu['values']=('-',
    #                       'example',
    #                       'jim')
    # names_label.pack(side=tk.LEFT, padx=5)
    # names_menu.pack(side=tk.LEFT)

    # frame_toggled=tk.BooleanVar(value=False)
    # table_button = tk.Button(center, text='create table', command=lambda: form_table(frame_toggled,below_center,lst))

    # # toggled elements
    
    # employeelist=[orderframe, employee_time_button, employee_skills_button,table_button]


    # #add manager and employee buttons
    # manager_toggled=tk.BooleanVar(value=False)
    # employee_toggled=tk.BooleanVar(value=False)
    # edit_toggled=tk.BooleanVar(value=False)

    # toggledlist=[manager_toggled,employee_toggled,edit_toggled]

    

    # start_center.pack(pady= 10)
    
    # manager=tk.Button(start_center, text="Manager", command=lambda:toggle_elements(manager_toggled,employee_toggled,managerlist,employeelist, Cancel_button))
    # manager.pack(side=tk.LEFT, padx= 5)

    # employee=tk.Button(start_center, text="Employee", command=lambda:toggle_elements(employee_toggled,manager_toggled,employeelist,managerlist, Cancel_button))
    # employee.pack(side=tk.LEFT)

    # edit_button=tk.Button(start_center, text="Edit")
    # edit_button.pack(padx=5)

    # center.pack(anchor='center')
    
    
    
    # editlist=[] #figure out what edit needs



    # Cancel_button.config(command=lambda: cancel(toggledlist,managerlist,employeelist, Cancel_button))

    

    # # Create the button
    
    # # Create the Entry widget (initially hidden)
    # entry_box = tk.Entry(root, width=30)
    # entered=entry_box.get() # stores string entered into the text box

    # Start the main event loop
    root.mainloop()
