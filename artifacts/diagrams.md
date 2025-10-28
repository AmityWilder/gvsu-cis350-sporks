# Gantt

```mermaid
gantt
    title Project Timeline
    dateFormat YYYY-M-D
    axisFormat %b %e
    tickInterval 1week
    weekday monday
    todayMarker on
    Start  : vert, 2025-9-2, 0d
    Finish : vert, 2025-12-13, 0d

    section Planning
    Initial planning                        : done, plan-init, 2025-9-2, 4w

    section GUI
    Learn tkinter                           : crit, active, lrn-tk, 2025-10-15, 7w
    Base visual layout                      : crit, active, base-layout, 2025-10-20, 4w
    Polish                                  : polish, after base-layout, 26d
    Server interop                          : crit, active, interop-clt, 2025-10-25, 2w
    Base functionality                      : crit, milestone, interop-clt
    Sign-in page                            : sign-in, after interop-clt, 1w
    User/manager permissions                : usr-perms, after sign-in, 1.5w
    Distinct employee and manager sides     : milestone, sign-in usr-perms
    Schedule display                        : crit, sch-disp, after usr-perms, 1w
    Schedule editing                        : crit, sch-edit, after usr-perms, 2w

    section Scheduler
    Basic types                             : crit, done, tys, 2025-9-17, 2025-9-23
    Learn clap                              : done, 2025-9-26, 2025-10-17
    CLI                                     : done, cli, 2025-9-23, 2025-10-17
    Learn miette                            : done, 2025-9-25, 2025-10-24
    Error display                           : done, errs, 2025-9-23, 2025-10-24
    Learn daggy                             : done, 2025-10-22, 2025-10-26
    Deadline ordering                       : crit, done, dl-ord, 2025-9-23, 2025-10-25
    Learn xml-rpc                           : done, 2025-10-25, 1d
    Client interop                          : crit, active, interop-srv, 2025-10-25, 2w
    From availability                       : crit, active, alg-avail, after dl-ord, 3w
    Availability rules                      : avail-rules, after alg-avail, 1w
    Skill requirements                      : skill-req, after avail-rules, 2w
    Overrides                               : ovr, after skill-req, 1w
    Algorithm                               : crit, milestone, dl-ord alg-avail avail-rules skill-req ovr

    section Saving
    Requirement data                        : crit, active, ser-dat, after dl-ord, 7w
    Schedule                                : ser-sch, after alg-avail, 3w
    File streaming                          : ser-stream, after alg-avail, 3w

    section Loading
    Requirement data                        : crit, active, de-dat, after dl-ord, 7w
    Schedule                                : de-sch, after alg-avail, 3w
    File streaming                          : de-stream, after alg-avail, 3w

    section Importing
    Data from JSON                          : im-json, after alg-avail, 4w
    Data from CSV                           : im-csv, after alg-avail, 4w

    section Exporting
    PNG/JPEG                                : ex-img, after alg-avail, 4w
    PDF                                     : ex-pdf, after alg-avail, 4w
    ical                                    : ex-ical, after alg-avail, 4w
```

# Requirements
```mermaid
requirementDiagram

requirement req {

}

element scheduler {

}

scheduler - satisfies -> req
```

# Flowchart
```mermaid
flowchart TD

    start@{shape: start}
    -->
    uom[user vs manager]@{shape: decision}
    uom --> manager --> generate
    uom --> user --> availability@{shape: manual-input}
```

# Components
```mermaid
classDiagram
    class UserId {
        -u32
    }
    class TaskId {
        -u64
    }
    class Slot {
        +Range~DateTime~ time
    }
    class User {
        ~UserId id
        +String name
        +Availability availability
        +Set~Skill~ skills
        +id() UserId
    }
    class Task {
        ~TaskId id
        +String title
        +List~Skill~ skill_reqs
        +id() TaskId
    }
```
