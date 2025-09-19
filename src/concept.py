from datetime import datetime, date, time, timedelta
from enum import Enum, Flag, auto


type UserID = int


class Interval:
    start: datetime | None
    """
    The beginning of the interval

    `datetime`
        Interval starts at <datetime>
    `None`
        For all of history
    """

    end: datetime | None
    """
    The conclusion of the interval

    `datetime`
        Interval ends at <datetime>
    `None`:
        into perpetuity
    """

    def __init__(self, *, start: datetime | None = None, end: datetime | None = None) -> None:
        self.start = start
        self.end = end

    def duration(self) -> timedelta | None:
        """
        Returns `None` if perpetual in either direction
        """
        if self.start is not None and self.end is not None:
            return self.end - self.start
        else:
            return None


class Weekdays(Flag):
    """A collection of weekly bitflgs."""
    Monday = auto()
    Tuesday = auto()
    Wednesday = auto()
    Thursday = auto()
    Friday = auto()
    Saturday = auto()
    Sunday = auto()


class Weekday(Enum):
    """A single day of the week."""
    Monday = 1
    Tuesday = 2
    Wednesday = 3
    Thursday = 4
    Friday = 5
    Saturday = 6
    Sunday = 7

    def as_flag(self) -> Weekdays:
        return Weekdays(1 << self.value)


class Day(int):
    """A day of the month."""


class Months(Flag):
    """A collection of month bitflags."""
    January = auto()
    February = auto()
    March = auto()
    April = auto()
    May = auto()
    June = auto()
    July = auto()
    August = auto()
    September = auto()
    October = auto()
    November = auto()
    December = auto()


class Month(Enum):
    """A single month."""
    January = 1
    February = 2
    March = 3
    April = 4
    May = 5
    June = 6
    July = 7
    August = 8
    September = 9
    October = 10
    November = 11
    December = 12

    def as_flag(self) -> Months:
        return Months(1 << self.value)


class RepeatingTime:
    days: list[tuple[Months, Weekdays]]
    months: Months
    time: time | None

    lifetime: Interval
    """The interval over which the rule repeats."""


class Rule:
    """A rule for scheduling."""

    duration: timedelta
    """The duration of the slot(s) that the rule applies to."""


class User:
    """A person who can be scheduled to work on a task."""

    id: UserID
    """Unique identifier for distinguishing this user from all others."""

    name: str | None
    """Display name for representing the user on the manager-facing UI. Can be changed without changing `id`."""

    availability: list[tuple[Rule, RepeatingTime, time | None, time | None]]
    """
    Rules regarding when the user can or can't be scheduled.

    Examples
    --------
    - "available every Monday 3pm-7pm",
    - "never available on Fridays"
    """

    user_prefs: dict[UserID, float]
    """
    Scale of [-1.0 .. 1.0] for preference towards other users.

    Range
    -----
    `INFINITY`
        **Always** schedule together (ex: handler).
        If unable to be scheduled *together*, **do not schedule *this* user (`self`).**
    `1.0`
        Maximize scheduling together.
        Only schedule separately if no other option.
    `0.0`
        No preference
        (equivalent to not being listed at all; which should be preferred for storage reasons)
    `-1.0`
        Minimize scheduling together.
        Only schedule together if no other option.
    `-INFINITY`
        **Never** schedule together (ex: restraining order).
        If unable to be scheduled *separately*, **do not schedule *that* user (user in dict).**
    """

    def __init__(self, id: UserID, name: str | None = None):
        self.id = id
        self.name = name


class Task:
    """A product or service to be completed."""

    title: str
    desc: str | None

    def __init__(self, title: str, desc: str | None = None):
        self.title = title
        self.desc = desc


class Slot:
    """A segment of time that can be allocated for work, such as a "shift"."""

    start: time
    duration: timedelta

    def __init__(self, start: time, duration: timedelta):
        self.start = start
        self.duration = duration



class Calandar:
    """A collection of all available time slots."""

    tasks: list[Task]
    slots: list[Slot]

    def __init__(self, start: time, duration: timedelta):
        self.start = start
        self.duration = duration


class Schedule:
    slots: list[tuple[Slot, set[Task], set[UserID]]]


def schedule(c: Calandar, users: list[UserID], user_data: dict[UserID, User]) -> Schedule:
    """Generate a schedule"""

    s = Schedule()

    # TODO: Fill schedule with users

    return s
