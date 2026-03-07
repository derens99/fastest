"""Check 11: setup/teardown methods in classes."""

_setup_teardown_log = []


class TestSetupTeardown:
    def setup_method(self):
        _setup_teardown_log.append("setup")

    def teardown_method(self):
        _setup_teardown_log.append("teardown")

    def test_first(self):
        assert True

    def test_second(self):
        assert True


class TestClassSetup:
    items = []

    @classmethod
    def setup_class(cls):
        cls.items = [1, 2, 3]

    def test_items_exist(self):
        assert len(self.items) == 3

    def test_items_content(self):
        assert self.items == [1, 2, 3]
