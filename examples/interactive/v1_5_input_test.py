# Test for input() - Interactive, user will test manually

def test_input_no_prompt() -> None:
    """BLT-001: input() - reads user input"""
    name: str = input()
    print("You entered:", name)


def test_input_with_prompt() -> None:
    """BLT-001: input(prompt) - shows prompt then reads input"""
    name: str = input("Enter your name: ")
    print("Hello,", name)


def main() -> None:
    test_input_with_prompt()


main()
