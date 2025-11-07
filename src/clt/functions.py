import time, xmlrpc.client, subprocess, atexit, tkinter as tk


def updateScrollRegion(canvas,frame):
	canvas.update_idletasks()
	canvas.config(scrollregion=frame.bbox())

def form_table(canvas,frame,list):
    
    list.append(('','','','',))
    total_rows = len(list)
    total_columns = len(list[0])
    # if toggle.get():  # If textbox is currently visible
    #     frame.pack_forget()
    #     toggle.set(False)
        
    
    # show elements
    # else:  # If textbox is currently hidden
        
    for i in range(total_rows):
        for j in range(total_columns):
                
            e = tk.Entry(frame ,width=20, fg='blue',
                               font=('Arial',12,'bold'))
                
            e.grid(row=i, column=j)
            e.insert(tk.END, list[i][j])
    
    updateScrollRegion(canvas,frame)


def cancel(visiblelist, element, opp_element, Cancel):
    for i in range(len(element)):
        current=element[i]
        current.pack_forget()  # Hide the textbox
    
    for n in range(len(opp_element)):
        current=opp_element[n]
        current.pack_forget()  # Hide the textbox
    for j in range(len(visiblelist)):
        current=visiblelist[j]
        current.set(False)
    Cancel.pack_forget()

def add_task(proxy, title_box):
    title=title_box.get()
    title_box.delete(0,tk.END)
    added = proxy.add_tasks({'to_add': [{'title': title}]})

def add_user(proxy):
    
    added = proxy.add_users({'to_add': [{'name': 'example'}]})

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
        