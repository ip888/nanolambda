"""
Data Processing Handler - ETL pipeline operations
Use case: Transform, aggregate, and analyze data streams
"""
import json
import statistics
from datetime import datetime, timedelta


def handler(event, context):
    """
    Process and analyze data batches
    
    Operations:
    - Data validation and cleaning
    - Statistical analysis
    - Aggregation and grouping
    - Outlier detection
    """
    
    operation = event.get('operation', 'analyze')
    data = event.get('data', [])
    
    try:
        if operation == 'analyze':
            # Statistical analysis
            if not data:
                return {
                    'statusCode': 400,
                    'body': json.dumps({'error': 'No data provided'})
                }
            
            numeric_data = [float(x) for x in data if isinstance(x, (int, float))]
            
            if not numeric_data:
                return {
                    'statusCode': 400,
                    'body': json.dumps({'error': 'No numeric data found'})
                }
            
            analysis = {
                'count': len(numeric_data),
                'mean': statistics.mean(numeric_data),
                'median': statistics.median(numeric_data),
                'stdev': statistics.stdev(numeric_data) if len(numeric_data) > 1 else 0,
                'min': min(numeric_data),
                'max': max(numeric_data),
                'sum': sum(numeric_data)
            }
            
            # Detect outliers (values > 2 standard deviations from mean)
            if len(numeric_data) > 1:
                mean = analysis['mean']
                stdev = analysis['stdev']
                outliers = [x for x in numeric_data if abs(x - mean) > 2 * stdev]
                analysis['outliers'] = outliers
                analysis['outlier_count'] = len(outliers)
            
            return {
                'statusCode': 200,
                'body': json.dumps(analysis)
            }
        
        elif operation == 'aggregate':
            # Group and aggregate data
            records = event.get('records', [])
            group_by = event.get('group_by', 'category')
            
            groups = {}
            for record in records:
                key = record.get(group_by, 'unknown')
                if key not in groups:
                    groups[key] = []
                groups[key].append(record)
            
            aggregated = {}
            for key, items in groups.items():
                aggregated[key] = {
                    'count': len(items),
                    'total': sum(item.get('value', 0) for item in items),
                    'average': sum(item.get('value', 0) for item in items) / len(items)
                }
            
            return {
                'statusCode': 200,
                'body': json.dumps(aggregated)
            }
        
        elif operation == 'clean':
            # Data cleaning and validation
            records = event.get('records', [])
            rules = event.get('rules', {})
            
            cleaned = []
            errors = []
            
            for i, record in enumerate(records):
                try:
                    # Validate required fields
                    if 'required_fields' in rules:
                        for field in rules['required_fields']:
                            if field not in record or record[field] is None:
                                raise ValueError(f"Missing required field: {field}")
                    
                    # Type validation
                    if 'types' in rules:
                        for field, expected_type in rules['types'].items():
                            if field in record:
                                if expected_type == 'int':
                                    record[field] = int(record[field])
                                elif expected_type == 'float':
                                    record[field] = float(record[field])
                                elif expected_type == 'str':
                                    record[field] = str(record[field])
                    
                    # Range validation
                    if 'ranges' in rules:
                        for field, (min_val, max_val) in rules['ranges'].items():
                            if field in record:
                                val = record[field]
                                if val < min_val or val > max_val:
                                    raise ValueError(f"{field} out of range: {val}")
                    
                    cleaned.append(record)
                
                except Exception as e:
                    errors.append({'index': i, 'error': str(e), 'record': record})
            
            return {
                'statusCode': 200,
                'body': json.dumps({
                    'cleaned': cleaned,
                    'errors': errors,
                    'success_rate': len(cleaned) / len(records) * 100
                })
            }
        
        else:
            return {
                'statusCode': 400,
                'body': json.dumps({'error': f'Unknown operation: {operation}'})
            }
    
    except Exception as e:
        return {
            'statusCode': 500,
            'body': json.dumps({'error': str(e)})
        }
