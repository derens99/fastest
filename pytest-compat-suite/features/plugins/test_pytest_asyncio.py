"""
Tests for pytest-asyncio compatibility.
Tests async/await test support and event loop handling.
"""
import pytest
import asyncio
import time
from typing import AsyncGenerator, AsyncIterator


class TestAsyncioBasic:
    """Test basic asyncio functionality."""
    
    @pytest.mark.asyncio
    async def test_async_function(self):
        """Test basic async test function."""
        await asyncio.sleep(0.1)
        result = await self.async_operation()
        assert result == 42
    
    async def async_operation(self):
        """Helper async function."""
        await asyncio.sleep(0.05)
        return 42
    
    @pytest.mark.asyncio
    async def test_multiple_awaits(self):
        """Test multiple await operations."""
        result1 = await self.fetch_data(1)
        result2 = await self.fetch_data(2)
        result3 = await self.fetch_data(3)
        assert result1 + result2 + result3 == 6
    
    async def fetch_data(self, value):
        """Simulate async data fetching."""
        await asyncio.sleep(0.01)
        return value
    
    @pytest.mark.asyncio
    async def test_async_context_manager(self):
        """Test async context managers."""
        async with self.async_resource() as resource:
            assert resource == "resource"
    
    async def async_resource(self):
        """Async context manager for testing."""
        class AsyncResource:
            async def __aenter__(self):
                await asyncio.sleep(0.01)
                return "resource"
            
            async def __aexit__(self, exc_type, exc_val, exc_tb):
                await asyncio.sleep(0.01)
        
        return AsyncResource()


class TestAsyncioFixtures:
    """Test async fixtures."""
    
    @pytest.fixture
    async def async_fixture(self):
        """Async fixture example."""
        await asyncio.sleep(0.01)
        return {"async": "data"}
    
    @pytest.mark.asyncio
    async def test_with_async_fixture(self, async_fixture):
        """Test using async fixture."""
        assert async_fixture == {"async": "data"}
    
    @pytest.fixture
    async def async_yield_fixture(self):
        """Async yield fixture example."""
        await asyncio.sleep(0.01)
        resource = "async_resource"
        yield resource
        # Cleanup
        await asyncio.sleep(0.01)
    
    @pytest.mark.asyncio
    async def test_with_async_yield_fixture(self, async_yield_fixture):
        """Test using async yield fixture."""
        assert async_yield_fixture == "async_resource"
    
    @pytest.fixture(scope="session")
    async def async_session_fixture(self):
        """Async session-scoped fixture."""
        await asyncio.sleep(0.01)
        return "session_data"
    
    @pytest.mark.asyncio
    async def test_async_session_fixture(self, async_session_fixture):
        """Test async session fixture."""
        assert async_session_fixture == "session_data"


class TestAsyncioEventLoop:
    """Test event loop handling."""
    
    @pytest.mark.asyncio
    async def test_event_loop_fixture(self, event_loop):
        """Test event_loop fixture is available."""
        assert isinstance(event_loop, asyncio.AbstractEventLoop)
        assert event_loop.is_running()
    
    @pytest.mark.asyncio
    async def test_custom_event_loop_policy(self):
        """Test custom event loop policy."""
        loop = asyncio.get_event_loop()
        assert loop is not None
    
    @pytest.mark.asyncio
    async def test_event_loop_scope(self):
        """Test event loop scope handling."""
        loop1 = asyncio.get_event_loop()
        await asyncio.sleep(0.01)
        loop2 = asyncio.get_event_loop()
        assert loop1 is loop2


class TestAsyncioMarkers:
    """Test asyncio markers and configuration."""
    
    @pytest.mark.asyncio
    async def test_basic_asyncio_mark(self):
        """Test basic @pytest.mark.asyncio usage."""
        assert True
    
    @pytest.mark.asyncio(scope="session")
    async def test_session_scoped_async(self):
        """Test session-scoped async test."""
        await asyncio.sleep(0.01)
        assert True
    
    @pytest.mark.asyncio
    @pytest.mark.timeout(1)
    async def test_async_with_timeout(self):
        """Test async test with timeout."""
        await asyncio.sleep(0.1)
        assert True
    
    @pytest.mark.asyncio
    @pytest.mark.parametrize("value", [1, 2, 3])
    async def test_async_parametrized(self, value):
        """Test parametrized async tests."""
        await asyncio.sleep(0.01)
        assert value in [1, 2, 3]


class TestAsyncioExceptions:
    """Test exception handling in async tests."""
    
    @pytest.mark.asyncio
    async def test_async_exception(self):
        """Test exception in async test."""
        async def failing_operation():
            await asyncio.sleep(0.01)
            raise ValueError("Async error")
        
        with pytest.raises(ValueError, match="Async error"):
            await failing_operation()
    
    @pytest.mark.asyncio
    async def test_async_timeout_error(self):
        """Test asyncio timeout error."""
        async def slow_operation():
            await asyncio.sleep(10)
        
        with pytest.raises(asyncio.TimeoutError):
            await asyncio.wait_for(slow_operation(), timeout=0.1)
    
    @pytest.mark.asyncio
    async def test_cancelled_error(self):
        """Test handling cancelled tasks."""
        async def cancellable_operation():
            try:
                await asyncio.sleep(10)
            except asyncio.CancelledError:
                # Cleanup
                raise
        
        task = asyncio.create_task(cancellable_operation())
        await asyncio.sleep(0.01)
        task.cancel()
        
        with pytest.raises(asyncio.CancelledError):
            await task


class TestAsyncioUtilities:
    """Test asyncio utility functions."""
    
    @pytest.mark.asyncio
    async def test_gather_multiple_tasks(self):
        """Test gathering multiple async tasks."""
        async def task(n):
            await asyncio.sleep(0.01 * n)
            return n * 2
        
        results = await asyncio.gather(
            task(1),
            task(2),
            task(3)
        )
        assert results == [2, 4, 6]
    
    @pytest.mark.asyncio
    async def test_create_task(self):
        """Test creating and awaiting tasks."""
        async def background_task():
            await asyncio.sleep(0.01)
            return "completed"
        
        task = asyncio.create_task(background_task())
        result = await task
        assert result == "completed"
    
    @pytest.mark.asyncio
    async def test_async_generator(self):
        """Test async generators."""
        async def async_gen():
            for i in range(3):
                await asyncio.sleep(0.01)
                yield i
        
        values = []
        async for value in async_gen():
            values.append(value)
        
        assert values == [0, 1, 2]


class TestAsyncioMocking:
    """Test mocking in async tests."""
    
    @pytest.mark.asyncio
    async def test_mock_async_function(self, mocker):
        """Test mocking async functions."""
        async def original_func():
            await asyncio.sleep(1)
            return "original"
        
        mock_func = mocker.AsyncMock(return_value="mocked")
        mocker.patch('__main__.original_func', mock_func)
        
        result = await mock_func()
        assert result == "mocked"
        mock_func.assert_awaited_once()
    
    @pytest.mark.asyncio
    async def test_mock_async_context_manager(self, mocker):
        """Test mocking async context managers."""
        mock_cm = mocker.AsyncMock()
        mock_cm.__aenter__.return_value = "entered"
        mock_cm.__aexit__.return_value = None
        
        async with mock_cm as value:
            assert value == "entered"
        
        mock_cm.__aenter__.assert_awaited_once()
        mock_cm.__aexit__.assert_awaited_once()


class TestAsyncioIntegration:
    """Test asyncio integration with other features."""
    
    @pytest.mark.asyncio
    @pytest.mark.skipif(sys.platform == "win32", reason="Unix only")
    async def test_async_with_skipif(self):
        """Test async test with conditional skip."""
        await asyncio.sleep(0.01)
        assert True
    
    @pytest.mark.asyncio
    @pytest.mark.xfail(reason="Expected async failure")
    async def test_async_xfail(self):
        """Test async test with expected failure."""
        await asyncio.sleep(0.01)
        assert False
    
    @pytest.mark.asyncio
    async def test_async_with_tmp_path(self, tmp_path):
        """Test async test with tmp_path fixture."""
        test_file = tmp_path / "async_test.txt"
        
        async def write_async():
            await asyncio.sleep(0.01)
            test_file.write_text("async content")
        
        await write_async()
        assert test_file.read_text() == "async content"


class TestAsyncioClassBased:
    """Test class-based async tests."""
    
    async def async_setup(self):
        """Async setup method."""
        await asyncio.sleep(0.01)
        self.data = "initialized"
    
    async def async_teardown(self):
        """Async teardown method."""
        await asyncio.sleep(0.01)
        self.data = None
    
    @pytest.mark.asyncio
    async def test_with_async_setup(self):
        """Test that uses async setup."""
        await self.async_setup()
        assert self.data == "initialized"
        await self.async_teardown()


# Async fixtures for testing
@pytest.fixture
async def async_client():
    """Async client fixture."""
    class AsyncClient:
        async def request(self, url):
            await asyncio.sleep(0.01)
            return {"url": url, "status": 200}
    
    return AsyncClient()


@pytest.fixture
async def async_database():
    """Async database fixture."""
    class AsyncDB:
        async def connect(self):
            await asyncio.sleep(0.01)
            return self
        
        async def query(self, sql):
            await asyncio.sleep(0.01)
            return [{"id": 1, "name": "Test"}]
        
        async def close(self):
            await asyncio.sleep(0.01)
    
    db = AsyncDB()
    await db.connect()
    yield db
    await db.close()