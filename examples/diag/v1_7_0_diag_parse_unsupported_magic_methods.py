# Expect: parse-phase diagnostics (unsupported magic methods)
class A:
    def __getitem__(self, idx):
        pass

    def __setitem__(self, idx, value):
        pass

    def __len__(self):
        return 0

    def __contains__(self, item):
        return False
