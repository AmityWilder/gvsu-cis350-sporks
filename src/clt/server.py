import time, xmlrpc.client, subprocess, atexit, datetime
from typing import Literal, Iterable
from datetime import datetime
import marshal


def open_server(build: Literal["debug", "release"], socket: str = "127.0.0.1:8080"):
    """Open a server on the requested build, listening at the requested socket. Use as options:
    build=DEBUG or RELEASE - The build directory to search for the executable in (starting in target)
    socket=SocketAddr - The socket to listen at.
    """
    # open the server in parallel
    srv = subprocess.Popen([f"./target/{build}/gvsu-cis350-sporks", socket])
    time.sleep(0.01) # wait for the server to be open

    proxy = ProxyWrapper(f"http://{socket}")

    def close_server():
        print("attempting to close server")
        if srv.poll() is not None:
            print("server already closed")
        else:
            try:
                proxy.quit()
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
            except Exception as e:
                print(f"quit errored: {e}\nterminating server")
                srv.terminate()
            finally:
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
    return proxy


class Pattern(str):
    pass


class RuleId(int): pass
class SlotId(int): pass
class TaskId(int): pass
class UserId(int): pass


class TimeInterval:
    start: datetime
    end: datetime

    def __init__(
        self,
        *,
        start: datetime,
        end: datetime,
    ) -> None:
        self.start = start
        self.end = end


class Freq:
    seconds: int | None
    minutes: int | None
    hours: int | None
    days: int | None
    weeks: int | None
    months: int | None
    years: int | None

    def __init__(
        self,
        *,
        seconds: int | None = None,
        minutes: int | None = None,
        hours: int | None = None,
        days: int | None = None,
        weeks: int | None = None,
        months: int | None = None,
        years: int | None = None,
    ) -> None:
        self.seconds = seconds
        self.minutes = minutes
        self.hours = hours
        self.days = days
        self.weeks = weeks
        self.months = months
        self.years = years

class Rep:
    every: Freq
    start: datetime
    until: datetime | None

    def __init__(
        self,
        *,
        every: Freq,
        start: datetime,
        until: datetime | None = None,
    ) -> None:
        self.every = every
        self.start = start
        self.until = until


class Rule:
    include: list[TimeInterval]
    repeat: Rep | None
    preference: float

    def __init__(
        self,
        *,
        include: list[TimeInterval],
        repeat: Rep | None = None,
        preference: float,
    ) -> None:
        self.include = include
        self.repeat = repeat
        self.preference = preference


class Slot:
    start: datetime
    end: datetime
    min_staff: int | None
    name: str | None

    def __init__(
        self,
        *,
        start: datetime,
        end: datetime,
        min_staff: int | None = None,
        name: str | None = None,
    ) -> None:
        self.start = start
        self.end = end
        self.min_staff = min_staff
        self.name = name


class SlotFilter:
    starting_after: datetime | None
    starting_before: datetime | None
    ending_after: datetime | None
    ending_before: datetime | None
    min_staff_min: int | None
    min_staff_max: int | None
    name_pat: Pattern | None

    def __init__(
        self,
        *,
        starting_after: datetime | None = None,
        starting_before: datetime | None = None,
        ending_after: datetime | None = None,
        ending_before: datetime | None = None,
        min_staff_min: int | None = None,
        min_staff_max: int | None = None,
        name_pat: Pattern | None = None,
    ) -> None:
        self.starting_after = starting_after
        self.starting_before = starting_before
        self.ending_after = ending_after
        self.ending_before = ending_before
        self.min_staff_min = min_staff_min
        self.min_staff_max = min_staff_max
        self.name_pat = name_pat


class Task:
    title: str
    desc: str | None
    deadline: datetime | None
    awaiting: list[TaskId] | None

    def __init__(
        self,
        *,
        title: str,
        desc: str | None = None,
        deadline: datetime | None = None,
        awaiting: list[TaskId] | None = None,
    ) -> None:
        self.title = title
        self.desc = desc
        self.deadline = deadline
        self.awaiting = awaiting


class TaskFilter:
    ids: set[TaskId] | None
    title_pat: Pattern | None
    desc_pat: Pattern | None
    deadline_after: datetime | None
    deadline_before: datetime | None

    def __init__(
        self,
        *,
        ids: set[TaskId] | None = None,
        title_pat: Pattern | None = None,
        desc_pat: Pattern | None = None,
        deadline_after: datetime | None = None,
        deadline_before: datetime | None = None,
    ) -> None:
        self.ids = ids
        self.title_pat = title_pat
        self.desc_pat = desc_pat
        self.deadline_after = deadline_after
        self.deadline_before = deadline_before


class User:
    name: str

    def __init__(
        self,
        *,
        name: str
    ) -> None:
        self.name = name


class UserFilter:
    ids: list[UserId] | None
    name_pat: Pattern | None

    def __init__(
        self,
        *,
        ids: list[UserId] | None = None,
        name_pat: Pattern | None = None,
    ) -> None:
        self.ids = ids
        self.name_pat = name_pat


class ProxyWrapper(xmlrpc.client.ServerProxy):
    def sv_pat_starts_with(self, pat: str) -> Pattern | None:
        return self.pat_starts_with(pat)

    def sv_pat_ends_with(self, pat: str) -> Pattern | None:
        return self.pat_ends_with(pat)

    def sv_pat_contains(self, pat: str) -> Pattern | None:
        return self.pat_contains(pat)

    def sv_pat_exactly(self, pat: str) -> Pattern | None:
        return self.pat_exactly(pat)

    def sv_pat_regex(self, pat: str) -> Pattern | None:
        return self.pat_regex(pat)


    def sv_add_rules(self, rules: Iterable[Rule]) -> list[RuleId]:
        return self.add_rules(list(rules))

    def sv_add_slots(self, slots: Iterable[Slot]) -> None:
        return self.add_slots(list(slots))

    def sv_add_tasks(self, tasks: Iterable[Task]) -> list[TaskId]:
        return self.add_tasks(list(tasks))

    def sv_add_users(self, users: Iterable[User]) -> list[UserId]:
        return self.add_users(list(users))


    def sv_get_rules(self, user: UserId) -> dict[RuleId, Rule]:
        return self.get_rules(user)

    def sv_get_slots(self, filters: SlotFilter) -> dict[SlotId, Slot]:
        return self.get_slots(filters)

    def sv_get_tasks(self, filters: TaskFilter) -> dict[TaskId, Task]:
        return self.get_tasks(filters)

    def sv_get_users(self, filters: UserFilter) -> dict[UserId, User]:
        return self.get_users(filters)


    def sv_pop_rules(self, ids: Iterable[RuleId]) -> set[RuleId] | None:
        return self.pop_rules(list(ids))

    def sv_pop_slots(self, ids: Iterable[SlotId]) -> set[SlotId] | None:
        return self.pop_slots(list(ids))

    def sv_pop_tasks(self, ids: Iterable[TaskId]) -> set[TaskId] | None:
        return self.pop_tasks(list(ids))

    def sv_pop_users(self, ids: Iterable[UserId]) -> set[UserId] | None:
        return self.pop_users(list(ids))


    def sv_quit(self) -> None:
        self.quit({})
