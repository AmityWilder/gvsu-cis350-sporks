Team name: Sporks

Team members:
- Conor Buchkowski
- Henry Lachman

# Introduction

<!-- In 2-4 paragraphs, describe your project concept -->
<!-- Also define some loosely defined features of your project using bullet points -->

This will be a scheduling program. It will take employee preferences and scheduling needs to generate a schedule. It will use Python in combination with Rust to create the schedule.

The front-end will accept the user input and display generated schedules. Python will handle front-end GUI/IO and communicate with the Rust scheduling server which will run on the local machine in parallel with the Python interface.

- Schedule can be displayed
- Schedules can be saved/loaded
- Employees and managers provide requirements

  - time slots
  - deadlines
  - available tasks
  - employee availability

  for the program to use in generating the schedule

# Anticipated Technologies

<!-- What technologies are needed to build this project -->

- `tkinter` - Python GUI library
- `subprocess` - Python subprocess execution library (for running the Rust server in parallel with the Python front-end)
- `serde` - Rust serialization/deserialization library

# Method/Approach

<!-- What is your estimated "plan of attack" for developing this project -->

**Agile**

- Create tickets
- Use checklist for features/additions
- Works well for small groups

We are fairly certain things are going to change while we make this project as we learn more about the technologies and specific needs of the usecase, which agile is more suitable for.

# Estimated Timeline

<!-- Figure out what your major milestones for this project will be, including how long you anticipate it *may* take to reach that point -->

(GUI and scheduling algorithm will be worked on in parallel as they are done by separate team members)

- GUI
  - Base visual layout
  - Functionality
    - Communicate with local Rust server
  - Distinct employee and manager sides
    - Sign-in
    - User permissions
  - Polish

- Scheduling algorithm
  - Given availability (e.g. assume all employees have identical skills)
  - Accounting for deadlines tasks that rely on other tasks to be completed first (maximum flow)
  - Accounting for skill requirements of tasks and employee skills
  - Availability "rules" (e.g. "*every* wednesday from 3pm to 4pm")
  - User overrides

- Serialization
  - Exporting
    - Schedule calandar
    - Schedule visual
      - PNG/JPEG
      - PDF
      - ical
  - Saving
    - Requirements data
    - In-progress schedule
    - File streaming (Only edit specific parts of a file that have changed)
  - Loading
    - Requirements data
    - In-progress schedule
    - File streaming (Only load specific parts of a file that are relevant)
  - Importing
    - Requirement data from JSON
    - Requirement data from CSV
    - Rough schedule data from ical

# Anticipated Problems

<!-- Describe any problems you foresee that you will need to overcome -->

- Poor performance (both speed and memory) when communicating between Rust and Python, due to not sharing memory and needing to both encode and decode data passed through internal interface
- Working with new library will require learning and might not have the expected features
- Security is hard to get right
- Changing serialization format to reflect new features may become hard if not future-proofed at the onset

<!-- Remember this is a living document is expected to be changed as you make progress on your project. -->
