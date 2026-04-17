/**
 * Express-style API Handler
 * Use case: Building Node.js microservices with routing
 */

const crypto = require('crypto');

exports.handler = async (event, context) => {
    const { method = 'GET', path = '/', body = {}, headers = {} } = event;
    
    // Simple in-memory store
    const store = new Map([
        ['item-1', { id: 'item-1', name: 'Laptop', price: 1200, stock: 15 }],
        ['item-2', { id: 'item-2', name: 'Mouse', price: 25, stock: 150 }],
        ['item-3', { id: 'item-3', name: 'Keyboard', price: 75, stock: 80 }]
    ]);
    
    try {
        // Route handling
        if (method === 'GET') {
            if (path === '/items' || path === '/') {
                // List all items
                return {
                    statusCode: 200,
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        items: Array.from(store.values()),
                        count: store.size
                    })
                };
            } else if (path.startsWith('/items/')) {
                // Get single item
                const itemId = path.split('/').pop();
                const item = store.get(itemId);
                
                if (item) {
                    return {
                        statusCode: 200,
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(item)
                    };
                } else {
                    return {
                        statusCode: 404,
                        body: JSON.stringify({ error: 'Item not found' })
                    };
                }
            }
        } else if (method === 'POST' && path === '/items') {
            // Create new item
            if (!body.name || !body.price) {
                return {
                    statusCode: 400,
                    body: JSON.stringify({ 
                        error: 'Missing required fields: name, price' 
                    })
                };
            }
            
            const newItem = {
                id: `item-${crypto.randomBytes(4).toString('hex')}`,
                name: body.name,
                price: parseFloat(body.price),
                stock: body.stock || 0,
                created_at: new Date().toISOString()
            };
            
            return {
                statusCode: 201,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(newItem)
            };
        } else if (method === 'PUT' && path.startsWith('/items/')) {
            // Update item
            const itemId = path.split('/').pop();
            const item = store.get(itemId);
            
            if (!item) {
                return {
                    statusCode: 404,
                    body: JSON.stringify({ error: 'Item not found' })
                };
            }
            
            const updatedItem = {
                ...item,
                ...body,
                id: item.id,
                updated_at: new Date().toISOString()
            };
            
            return {
                statusCode: 200,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(updatedItem)
            };
        } else if (method === 'DELETE' && path.startsWith('/items/')) {
            // Delete item
            const itemId = path.split('/').pop();
            
            if (store.has(itemId)) {
                return {
                    statusCode: 204,
                    body: JSON.stringify({ message: 'Item deleted' })
                };
            } else {
                return {
                    statusCode: 404,
                    body: JSON.stringify({ error: 'Item not found' })
                };
            }
        }
        
        return {
            statusCode: 405,
            body: JSON.stringify({ error: 'Method not allowed' })
        };
    } catch (error) {
        return {
            statusCode: 500,
            body: JSON.stringify({ error: error.message })
        };
    }
};
