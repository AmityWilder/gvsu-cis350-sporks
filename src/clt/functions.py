import time, xmlrpc.client, subprocess, atexit, tkinter as tk
from tkinter import ttk


def updateScrollRegion(canvas,frame):
	canvas.update_idletasks()
	canvas.config(scrollregion=frame.bbox())

class EntryRow:
    def __init__(self, frame, row_index, on_remove_callback,length):
        self.frame = frame
        self.row_index = row_index
        self.on_remove_callback = on_remove_callback
        self.widgets = []

        # Create Entry widgets for this row
        for j in range(length):
            self.entry1 = ttk.Entry(frame, width=20, foreground='blue',
                                font=('Arial',12))
            self.entry1.grid(row=self.row_index, column=j, padx=5, pady=2)
            self.widgets.append(self.entry1)


        # Remove button for this row
        self.remove_button = ttk.Button(frame, text="Remove", command=self.remove_self)
        self.remove_button.grid(row=self.row_index, column=length, padx=5, pady=2)
        self.widgets.append(self.remove_button)

    def remove_self(self):
        # Destroy all widgets in this row
        for widget in self.widgets:
            widget.destroy()
        self.on_remove_callback(self) # Notify the main application to update row indices

    def get_data(self):
        data=[]
        for i in range(len(self.widgets)-1):
            data.append(self.widgets[i].get())
        return data


class Table:
    def __init__(self, frame,canvas,outframe,labels):
        self.frame = frame
        self.canvas=canvas
        self.length=len(labels)

        self.rows = []
        self.current_row_index = 0

        self.add_button = ttk.Button(outframe, text="Add Row", command=self.add_row)
        self.add_button.grid(row=0, column=0, columnspan=self.length+1, pady=10)

        # Initial header
        for i in range(len(labels)):

            tk.Label(self.frame, text=labels[i],font=('Arial',12,'bold'),foreground='blue').grid(row=0, column=i, padx=50)

        #self.add_row() # Add an initial row

    def add_row(self):
        new_row = EntryRow(self.frame, self.current_row_index+1, self.on_row_removed,self.length)
        self.rows.append(new_row)
        self.current_row_index += 1
        updateScrollRegion(self.canvas,self.frame)

    def on_row_removed(self, removed_row):
        self.rows.remove(removed_row)
        self.reindex_rows()

    def reindex_rows(self):
        # Update row_index for all remaining rows and re-grid them
        for i, row_obj in enumerate(self.rows):
            row_obj.row_index = i+1 # +2 to account for header and add button
            for j, widget in enumerate(row_obj.widgets):
                widget.grid(row=row_obj.row_index, column=j, padx=5, pady=2)

        self.current_row_index = len(self.rows)
        updateScrollRegion(self.canvas,self.frame)
    def send(self):
        data=[]
        for i in self.rows:
            data.append(i.get_data())
        return data


    


        





# def cancel(visiblelist, element, opp_element, Cancel):
#     for i in range(len(element)):
#         current=element[i]
#         current.pack_forget()  # Hide the textbox
    
#     for n in range(len(opp_element)):
#         current=opp_element[n]
#         current.pack_forget()  # Hide the textbox
#     for j in range(len(visiblelist)):
#         current=visiblelist[j]
#         current.set(False)
#     Cancel.pack_forget()

def saving(proxy,table,savetype):
    data=table.send()
    if savetype=='shift':
        for i in data:
            add_shift(proxy,i)
    elif savetype=='task':
        for i in data:
            add_task(proxy,i)
    elif savetype == 'user':
        for i in data:
            add_user(proxy,i)


def add_shift(proxy, box):
    start=box[1]
    end = box[2]
    staff=box[3]
    name=box[0]
    added=proxy.add_slots({'to_add': [{'start': start,
                                       'end': end,
                                       'min_staff': staff,
                                       'name': name}]})

def add_task(proxy, title_box):
    title=title_box[0]
    desc=title_box[2]
    datetime=title_box[1]
    added = proxy.add_tasks({'to_add': [{'title': title,
                                         'desc': desc,
                                         'deadline': datetime}]})

def add_user(proxy, box):
    name=box[0]
    added = proxy.add_users({'to_add': [{'name': name}]})

# def toggle_elements(curr_element_visible,opp_element_visible, element, opp_element, Cancel):
#     # hide elements
#     if curr_element_visible.get():  # If textbox is currently visible
#         for i in range(len(element)):
#             current=element[i]
#             current.pack_forget()  # Hide the textbox
#         curr_element_visible.set(False)
#         Cancel.pack_forget()
    
#     # show elements
#     else:  # If textbox is currently hidden
#         if opp_element_visible.get():
#             for n in range(len(opp_element)):
#                 current=opp_element[n]
#                 current.pack_forget()  # Hide the textbox
#             opp_element_visible.set(False)
#         for j in range(len(element)):
#             current=element[j]
#             current.pack(pady=5)  # Show the textbox
#         curr_element_visible.set(True)
#         Cancel.pack(side=tk.TOP)
        