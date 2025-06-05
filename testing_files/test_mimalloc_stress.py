
import pytest
import json
import random
import string
from typing import List, Dict, Any

# Generate large data structures to stress the allocator
def generate_large_data(size: int = 1000) -> Dict[str, Any]:
    """Generate large data structures for allocation stress testing"""
    return {
        'id': ''.join(random.choices(string.ascii_letters, k=20)),
        'data': [
            {
                'key': f'item_{i}',
                'value': ''.join(random.choices(string.ascii_letters, k=50)),
                'nested': {
                    'array': list(range(100)),
                    'dict': {f'nested_key_{j}': f'nested_value_{j}' for j in range(20)}
                }
            } for i in range(size)
        ],
        'metadata': {
            'created_at': time.time(),
            'size': size,
            'checksum': hash(str(size))
        }
    }

# Session-scoped fixture with heavy allocation
@pytest.fixture(scope="session")
def session_data():
    """Session fixture that allocates significant memory"""
    print("🔧 Setting up session fixture with heavy allocation...")
    data = {
        'large_datasets': [generate_large_data(500) for _ in range(10)],
        'configuration': {f'config_{i}': f'value_{i}' for i in range(1000)},
        'cache': {}
    }
    return data

# Module-scoped fixture
@pytest.fixture(scope="module")
def module_data(session_data):
    """Module fixture that processes session data"""
    print("🔧 Setting up module fixture...")
    processed_data = {
        'processed': True,
        'source_count': len(session_data['large_datasets']),
        'aggregated': [
            {
                'dataset_id': i,
                'item_count': len(dataset['data']),
                'sample': dataset['data'][:5]  # Take first 5 items
            } for i, dataset in enumerate(session_data['large_datasets'])
        ]
    }
    return processed_data

# Class-based fixtures for comprehensive testing
class TestAllocatorPerformance:
    
    @pytest.fixture(scope="class")
    def class_fixture(self, module_data):
        """Class fixture with memory-intensive operations"""
        print("🔧 Setting up class fixture...")
        return {
            'class_data': generate_large_data(200),
            'module_summary': module_data['aggregated'],
            'instance_id': ''.join(random.choices(string.ascii_letters, k=10))
        }
    
    # Parametrized tests to stress fixture allocation/deallocation
    @pytest.mark.parametrize("data_size", [50, 100, 200, 500])
    @pytest.mark.parametrize("complexity", ["simple", "medium", "complex"])
    def test_allocation_performance(self, class_fixture, data_size, complexity):
        """Test that stresses allocation with different parameters"""
        
        # Simulate different complexity levels
        if complexity == "simple":
            test_data = list(range(data_size))
        elif complexity == "medium":
            test_data = [generate_large_data(10) for _ in range(data_size // 10)]
        else:  # complex
            test_data = [generate_large_data(20) for _ in range(data_size // 20)]
        
        # Perform operations that stress memory allocation
        processed = []
        for item in test_data:
            if isinstance(item, dict):
                serialized = json.dumps(item)
                deserialized = json.loads(serialized)
                processed.append(deserialized)
            else:
                processed.append({'value': item, 'squared': item ** 2})
        
        # Verify the test worked
        assert len(processed) > 0
        assert class_fixture['instance_id'] is not None
        
        print(f"✅ Processed {len(processed)} items with {complexity} complexity")
    
    @pytest.mark.parametrize("iteration", range(10))
    def test_repeated_allocation(self, class_fixture, iteration):
        """Test repeated allocation/deallocation patterns"""
        
        # Create and destroy large data structures repeatedly
        for i in range(50):
            large_data = generate_large_data(100)
            json_data = json.dumps(large_data)
            parsed_data = json.loads(json_data)
            
            # Verify data integrity
            assert parsed_data['metadata']['size'] == 100
            assert len(parsed_data['data']) == 100
            
            # Force deallocation by removing references
            del large_data, json_data, parsed_data
        
        print(f"✅ Completed iteration {iteration} with repeated allocations")

# Additional stress tests
class TestMemoryIntensiveOperations:
    
    @pytest.fixture
    def memory_intensive_fixture(self):
        """Fixture that creates memory pressure"""
        return {
            'large_arrays': [list(range(1000)) for _ in range(100)],
            'dictionaries': [{f'key_{i}_{j}': f'value_{i}_{j}' for j in range(100)} for i in range(50)],
            'json_data': [json.dumps(generate_large_data(50)) for _ in range(20)]
        }
    
    def test_memory_pressure(self, memory_intensive_fixture):
        """Test that creates significant memory pressure"""
        
        # Process all the data to ensure allocation
        total_items = 0
        for array in memory_intensive_fixture['large_arrays']:
            total_items += len(array)
        
        for dictionary in memory_intensive_fixture['dictionaries']:
            total_items += len(dictionary)
        
        for json_str in memory_intensive_fixture['json_data']:
            parsed = json.loads(json_str)
            total_items += len(parsed['data'])
        
        assert total_items > 0
        print(f"✅ Processed {total_items} total items under memory pressure")
    
    @pytest.mark.parametrize("batch_size", [10, 50, 100])
    def test_batch_processing(self, memory_intensive_fixture, batch_size):
        """Test batch processing with different sizes"""
        
        batches = []
        current_batch = []
        
        for i in range(500):
            item = {
                'id': i,
                'data': ''.join(random.choices(string.ascii_letters, k=100)),
                'metadata': generate_large_data(10)
            }
            current_batch.append(item)
            
            if len(current_batch) >= batch_size:
                # Process batch
                batch_json = json.dumps(current_batch)
                batch_parsed = json.loads(batch_json)
                batches.append(batch_parsed)
                current_batch = []
        
        # Process final batch
        if current_batch:
            batch_json = json.dumps(current_batch)
            batch_parsed = json.loads(batch_json)
            batches.append(batch_parsed)
        
        assert len(batches) > 0
        total_processed = sum(len(batch) for batch in batches)
        assert total_processed == 500
        
        print(f"✅ Processed 500 items in {len(batches)} batches of size {batch_size}")
