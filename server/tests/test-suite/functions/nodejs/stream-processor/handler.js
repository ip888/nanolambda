/**
 * Stream Processing Handler
 * Use case: Real-time data stream transformation and filtering
 */

exports.handler = async (event, context) => {
    const { operation = 'process', stream = [], config = {} } = event;
    
    try {
        if (operation === 'process') {
            // Process stream of events
            const results = [];
            let processed = 0;
            let filtered = 0;
            let errors = 0;
            
            for (const item of stream) {
                try {
                    // Apply filters
                    if (config.filters) {
                        let passedFilters = true;
                        
                        for (const [key, value] of Object.entries(config.filters)) {
                            if (item[key] !== value) {
                                passedFilters = false;
                                break;
                            }
                        }
                        
                        if (!passedFilters) {
                            filtered++;
                            continue;
                        }
                    }
                    
                    // Transform item
                    let transformed = { ...item };
                    
                    if (config.transformations) {
                        for (const [key, transform] of Object.entries(config.transformations)) {
                            if (transform === 'uppercase' && typeof transformed[key] === 'string') {
                                transformed[key] = transformed[key].toUpperCase();
                            } else if (transform === 'lowercase' && typeof transformed[key] === 'string') {
                                transformed[key] = transformed[key].toLowerCase();
                            } else if (transform === 'multiply' && typeof transformed[key] === 'number') {
                                transformed[key] = transformed[key] * (config.multiplier || 2);
                            }
                        }
                    }
                    
                    // Add metadata
                    transformed._processed_at = new Date().toISOString();
                    transformed._processed_by = 'stream-processor';
                    
                    results.push(transformed);
                    processed++;
                    
                } catch (err) {
                    errors++;
                }
            }
            
            return {
                statusCode: 200,
                body: JSON.stringify({
                    results,
                    statistics: {
                        total: stream.length,
                        processed,
                        filtered,
                        errors,
                        success_rate: (processed / stream.length * 100).toFixed(2) + '%'
                    }
                })
            };
        } else if (operation === 'aggregate') {
            // Aggregate stream data
            const aggregates = {};
            const groupBy = config.groupBy || 'type';
            
            for (const item of stream) {
                const key = item[groupBy] || 'unknown';
                
                if (!aggregates[key]) {
                    aggregates[key] = {
                        count: 0,
                        sum: 0,
                        values: []
                    };
                }
                
                aggregates[key].count++;
                
                if (typeof item.value === 'number') {
                    aggregates[key].sum += item.value;
                    aggregates[key].values.push(item.value);
                }
            }
            
            // Calculate averages
            for (const key in aggregates) {
                if (aggregates[key].values.length > 0) {
                    aggregates[key].average = 
                        aggregates[key].sum / aggregates[key].values.length;
                    aggregates[key].min = Math.min(...aggregates[key].values);
                    aggregates[key].max = Math.max(...aggregates[key].values);
                }
                delete aggregates[key].values; // Remove raw values
            }
            
            return {
                statusCode: 200,
                body: JSON.stringify({
                    aggregates,
                    total_items: stream.length,
                    unique_groups: Object.keys(aggregates).length
                })
            };
        } else if (operation === 'window') {
            // Windowing operations
            const windowSize = config.windowSize || 10;
            const windows = [];
            
            for (let i = 0; i < stream.length; i += windowSize) {
                const window = stream.slice(i, i + windowSize);
                const windowStats = {
                    window_id: Math.floor(i / windowSize),
                    size: window.length,
                    start_index: i,
                    end_index: Math.min(i + windowSize - 1, stream.length - 1)
                };
                
                // Calculate window statistics
                const values = window
                    .map(item => item.value)
                    .filter(v => typeof v === 'number');
                
                if (values.length > 0) {
                    windowStats.sum = values.reduce((a, b) => a + b, 0);
                    windowStats.average = windowStats.sum / values.length;
                    windowStats.min = Math.min(...values);
                    windowStats.max = Math.max(...values);
                }
                
                windows.push(windowStats);
            }
            
            return {
                statusCode: 200,
                body: JSON.stringify({
                    windows,
                    window_size: windowSize,
                    total_windows: windows.length
                })
            };
        }
        
        return {
            statusCode: 400,
            body: JSON.stringify({ error: 'Unknown operation' })
        };
        
    } catch (error) {
        return {
            statusCode: 500,
            body: JSON.stringify({ error: error.message })
        };
    }
};
