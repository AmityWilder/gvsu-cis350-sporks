from server import *


address = 8080

def open_test_server():
    """Choose the next socket address so tests running in parallel dont try to share a server"""
    global address
    addr = address
    address += 1
    return open_server("debug", f"127.0.0.1:{addr}")


def test_pat_starts_with():
    with open_test_server() as proxy:
        pat = proxy.sv_pat_starts_with("squeak")
        assert pat is not None


def test_add_zero_users():
    with open_test_server() as proxy:
        ids = proxy.sv_add_users([])
        assert ids is not None and len(ids) == 0


def test_add_one_user():
    with open_test_server() as proxy:
        user = User(
            name = "Squeeby Deeby",
        )
        ids = proxy.sv_add_users([user])
        assert ids is not None and len(ids) == 1


def test_add_multiple_users():
    with open_test_server() as proxy:
        user1 = User(
            name = "Dill Pickle",
        )
        user2 = User(
            name = "Mark Hapenstance",
        )
        ids = proxy.sv_add_users([user1, user2])
        assert ids is not None and len(ids) == 2


def test_add_zero_slots():
    with open_test_server() as proxy:
        proxy.sv_add_slots([])


def test_add_one_slot():
    with open_test_server() as proxy:
        slot = Slot(
            start = datetime(2006, 11, 21, 16, 30),
            end = datetime(2006, 11, 21, 16, 45),
            min_staff = 3,
            name = "Morning Shift",
        )
        proxy.sv_add_slots([slot])


def test_add_multiple_slots():
    with open_test_server() as proxy:
        slot1 = Slot(
            start = datetime(2006, 11, 21, 16, 30),
            end = datetime(2006, 11, 21, 16, 45),
            min_staff = 3,
            name = "Morning Shift",
        )
        slot2 = Slot(
            start = datetime(2006, 11, 22, 13, 30),
            end = datetime(2006, 11, 23, 15, 00),
            min_staff = 2,
            name = "Noon Shift",
        )
        proxy.sv_add_slots([slot1, slot2])


def test_add_zero_tasks():
    with open_test_server() as proxy:
        proxy.sv_add_tasks([])


def test_add_one_task():
    with open_test_server() as proxy:
        task = Task(
            title = "Sweep dishes",
            desc = "you heard me",
            deadline = datetime(2006, 11, 23, 15, 00),
            awaiting = [],
        )
        proxy.sv_add_tasks([task])


def test_add_multiple_tasks():
    with open_test_server() as proxy:
        task1 = Task(
            title = "Sweep dishes",
            desc = "you heard me",
            deadline = datetime(2006, 11, 23, 15, 00),
            awaiting = [],
        )
        task2 = Task(
            title = "Build soap",
            desc = "you heard me",
            deadline = datetime(2006, 11, 23, 15, 00),
            awaiting = [],
        )
        proxy.sv_add_tasks([task1, task2])
