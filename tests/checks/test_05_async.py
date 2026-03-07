"""Check 5: Async test support."""

import asyncio


async def test_async_basic():
    await asyncio.sleep(0.01)
    assert True


async def test_async_gather():
    results = await asyncio.gather(
        asyncio.sleep(0.01),
        asyncio.sleep(0.01),
    )
    assert len(results) == 2


async def test_async_computation():
    async def compute(x):
        await asyncio.sleep(0.001)
        return x * 2

    result = await compute(21)
    assert result == 42


async def test_async_fail():
    await asyncio.sleep(0.01)
    assert 1 == 2, "async failure"


class TestAsyncClass:
    async def test_async_in_class(self):
        await asyncio.sleep(0.01)
        assert True

    async def test_async_class_fail(self):
        await asyncio.sleep(0.01)
        assert False, "async class failure"
