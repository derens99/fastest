"""
Tests for pytest-timeout compatibility.
Tests timeout functionality for test execution.
"""
import pytest
import time
import asyncio
import threading
from concurrent.futures import ThreadPoolExecutor


class TestTimeoutBasic:
    """Test basic timeout functionality."""
    
    @pytest.mark.timeout(1)
    def test_fast_test_passes(self):
        """Test that fast tests pass with timeout."""
        time.sleep(0.1)
        assert True
    
    @pytest.mark.timeout(0.5)
    def test_slow_test_fails(self):
        """Test that slow tests fail with timeout."""
        time.sleep(2)  # This should timeout
        assert True
    
    @pytest.mark.timeout(2)
    def test_exact_timing(self):
        """Test timeout with exact timing."""
        start = time.time()
        time.sleep(1.5)
        elapsed = time.time() - start
        assert elapsed < 2
    
    def test_no_timeout(self):
        """Test without timeout marker."""
        time.sleep(0.1)
        assert True


class TestTimeoutMethods:
    """Test different timeout methods."""
    
    @pytest.mark.timeout(1, method='thread')
    def test_thread_timeout(self):
        """Test timeout using thread method."""
        time.sleep(0.1)
        assert True
    
    @pytest.mark.timeout(1, method='signal')
    def test_signal_timeout(self):
        """Test timeout using signal method (Unix only)."""
        time.sleep(0.1)
        assert True
    
    @pytest.mark.timeout(1)
    def test_default_timeout_method(self):
        """Test default timeout method."""
        time.sleep(0.1)
        assert True


class TestTimeoutConfiguration:
    """Test timeout configuration options."""
    
    def test_global_timeout(self):
        """Test global timeout setting."""
        # Would be set via --timeout=X
        time.sleep(0.1)
        assert True
    
    @pytest.mark.timeout(2)
    def test_override_global_timeout(self):
        """Test marker overrides global timeout."""
        time.sleep(0.1)
        assert True
    
    def test_timeout_func_only(self):
        """Test --timeout-func-only option."""
        # Only function execution time counted
        time.sleep(0.1)
        assert True
    
    @pytest.mark.timeout(0)
    def test_disable_timeout(self):
        """Test timeout=0 disables timeout."""
        time.sleep(0.5)
        assert True


class TestTimeoutWithFixtures:
    """Test timeout with fixtures."""
    
    @pytest.fixture
    def slow_fixture(self):
        """Fixture that takes time."""
        time.sleep(0.2)
        return "fixture_data"
    
    @pytest.mark.timeout(1)
    def test_timeout_includes_fixtures(self, slow_fixture):
        """Test timeout includes fixture time."""
        assert slow_fixture == "fixture_data"
        time.sleep(0.1)
    
    @pytest.fixture
    def setup_teardown_fixture(self):
        """Fixture with setup and teardown."""
        time.sleep(0.1)  # Setup
        yield "data"
        time.sleep(0.1)  # Teardown
    
    @pytest.mark.timeout(1)
    def test_timeout_with_teardown(self, setup_teardown_fixture):
        """Test timeout includes teardown time."""
        assert setup_teardown_fixture == "data"


class TestTimeoutAsync:
    """Test timeout with async tests."""
    
    @pytest.mark.asyncio
    @pytest.mark.timeout(1)
    async def test_async_timeout(self):
        """Test timeout with async function."""
        await asyncio.sleep(0.1)
        assert True
    
    @pytest.mark.asyncio
    @pytest.mark.timeout(0.5)
    async def test_async_timeout_fails(self):
        """Test async timeout failure."""
        await asyncio.sleep(2)
        assert True
    
    @pytest.mark.asyncio
    @pytest.mark.timeout(2)
    async def test_async_multiple_awaits(self):
        """Test timeout with multiple awaits."""
        await asyncio.sleep(0.1)
        await asyncio.sleep(0.1)
        await asyncio.sleep(0.1)
        assert True


class TestTimeoutThreading:
    """Test timeout with threading."""
    
    @pytest.mark.timeout(2)
    def test_timeout_with_threads(self):
        """Test timeout with multiple threads."""
        def worker():
            time.sleep(0.1)
        
        threads = []
        for _ in range(5):
            t = threading.Thread(target=worker)
            t.start()
            threads.append(t)
        
        for t in threads:
            t.join()
        
        assert True
    
    @pytest.mark.timeout(1)
    def test_timeout_thread_pool(self):
        """Test timeout with thread pool."""
        def task(n):
            time.sleep(0.1)
            return n * 2
        
        with ThreadPoolExecutor(max_workers=3) as executor:
            futures = [executor.submit(task, i) for i in range(3)]
            results = [f.result() for f in futures]
        
        assert results == [0, 2, 4]


class TestTimeoutErrors:
    """Test timeout error handling."""
    
    @pytest.mark.timeout(0.5)
    def test_timeout_error_message(self):
        """Test timeout produces clear error message."""
        time.sleep(2)
        # Should show timeout error with duration
    
    @pytest.mark.timeout(1)
    def test_timeout_with_exception(self):
        """Test timeout when test raises exception."""
        time.sleep(0.1)
        raise ValueError("Test exception")
    
    @pytest.mark.timeout(1)
    def test_timeout_in_finally(self):
        """Test timeout with finally block."""
        try:
            time.sleep(0.1)
            assert True
        finally:
            time.sleep(0.1)  # Cleanup time


class TestTimeoutParametrized:
    """Test timeout with parametrized tests."""
    
    @pytest.mark.timeout(1)
    @pytest.mark.parametrize("sleep_time", [0.1, 0.2, 0.3])
    def test_parametrized_timeout(self, sleep_time):
        """Test timeout applies to each parameter."""
        time.sleep(sleep_time)
        assert sleep_time < 0.5
    
    @pytest.mark.parametrize("timeout,sleep_time", [
        pytest.param(1, 0.1, marks=pytest.mark.timeout(1)),
        pytest.param(0.5, 0.1, marks=pytest.mark.timeout(0.5)),
        pytest.param(2, 0.1, marks=pytest.mark.timeout(2)),
    ])
    def test_parametrized_different_timeouts(self, timeout, sleep_time):
        """Test different timeouts per parameter."""
        time.sleep(sleep_time)
        assert True


class TestTimeoutClassLevel:
    """Test class-level timeout settings."""
    
    @pytest.mark.timeout(2)
    class TestWithClassTimeout:
        """All methods in this class have 2s timeout."""
        
        def test_method_1(self):
            time.sleep(0.1)
            assert True
        
        def test_method_2(self):
            time.sleep(0.2)
            assert True
        
        @pytest.mark.timeout(1)
        def test_method_override(self):
            """Method timeout overrides class timeout."""
            time.sleep(0.1)
            assert True


class TestTimeoutDebugging:
    """Test timeout debugging features."""
    
    @pytest.mark.timeout(10)
    def test_timeout_traceback(self):
        """Test timeout shows where code was stuck."""
        def recursive_sleep(n):
            if n > 0:
                time.sleep(0.1)
                recursive_sleep(n - 1)
        
        recursive_sleep(100)  # Should timeout and show stack
    
    @pytest.mark.timeout(1)
    def test_timeout_with_debugger(self):
        """Test timeout behavior with debugger."""
        # Timeout might be disabled when debugging
        time.sleep(0.1)
        assert True


class TestTimeoutEdgeCases:
    """Test timeout edge cases."""
    
    @pytest.mark.timeout(1)
    def test_timeout_zero_sleep(self):
        """Test timeout with no sleep."""
        # Should pass immediately
        assert True
    
    @pytest.mark.timeout(0.1)
    def test_very_short_timeout(self):
        """Test very short timeout."""
        # Might fail due to overhead
        assert True
    
    @pytest.mark.timeout(3600)
    def test_very_long_timeout(self):
        """Test very long timeout (1 hour)."""
        time.sleep(0.1)
        assert True
    
    @pytest.mark.timeout(1)
    def test_timeout_with_recursion(self):
        """Test timeout with recursive calls."""
        def fibonacci(n):
            if n <= 1:
                return n
            time.sleep(0.001)
            return fibonacci(n-1) + fibonacci(n-2)
        
        result = fibonacci(10)
        assert result == 55


# Helper functions for timeout testing
def slow_function(duration):
    """Helper function that sleeps."""
    time.sleep(duration)
    return "completed"


def infinite_loop():
    """Helper function with infinite loop."""
    while True:
        time.sleep(0.1)