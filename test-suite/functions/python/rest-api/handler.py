"""
REST API Handler - Demonstrates HTTP request processing
Use case: Building RESTful microservices
"""
import json
import time
from datetime import datetime


def handler(event, context):
    """
    Process REST API requests with routing and validation
    
    Supports:
    - GET /users/:id - Retrieve user
    - POST /users - Create user
    - PUT /users/:id - Update user
    - DELETE /users/:id - Delete user
    """
    
    method = event.get('method', 'GET')
    path = event.get('path', '/')
    body = event.get('body', {})
    
    # Simulate in-memory database
    users_db = {
        '1': {'id': '1', 'name': 'Alice', 'email': 'alice@example.com'},
        '2': {'id': '2', 'name': 'Bob', 'email': 'bob@example.com'},
        '3': {'id': '3', 'name': 'Charlie', 'email': 'charlie@example.com'}
    }
    
    try:
        if method == 'GET':
            # GET /users/:id
            user_id = path.split('/')[-1]
            if user_id in users_db:
                return {
                    'statusCode': 200,
                    'headers': {'Content-Type': 'application/json'},
                    'body': json.dumps(users_db[user_id])
                }
            else:
                return {
                    'statusCode': 404,
                    'body': json.dumps({'error': 'User not found'})
                }
        
        elif method == 'POST':
            # POST /users - Create new user
            if not body.get('name') or not body.get('email'):
                return {
                    'statusCode': 400,
                    'body': json.dumps({'error': 'Missing required fields'})
                }
            
            new_id = str(len(users_db) + 1)
            new_user = {
                'id': new_id,
                'name': body['name'],
                'email': body['email'],
                'created_at': datetime.now().isoformat()
            }
            
            return {
                'statusCode': 201,
                'headers': {'Content-Type': 'application/json'},
                'body': json.dumps(new_user)
            }
        
        elif method == 'PUT':
            # PUT /users/:id - Update user
            user_id = path.split('/')[-1]
            if user_id not in users_db:
                return {
                    'statusCode': 404,
                    'body': json.dumps({'error': 'User not found'})
                }
            
            updated_user = users_db[user_id].copy()
            updated_user.update(body)
            updated_user['updated_at'] = datetime.now().isoformat()
            
            return {
                'statusCode': 200,
                'headers': {'Content-Type': 'application/json'},
                'body': json.dumps(updated_user)
            }
        
        elif method == 'DELETE':
            # DELETE /users/:id
            user_id = path.split('/')[-1]
            if user_id in users_db:
                return {
                    'statusCode': 204,
                    'body': json.dumps({'message': 'User deleted'})
                }
            else:
                return {
                    'statusCode': 404,
                    'body': json.dumps({'error': 'User not found'})
                }
        
        else:
            return {
                'statusCode': 405,
                'body': json.dumps({'error': 'Method not allowed'})
            }
    
    except Exception as e:
        return {
            'statusCode': 500,
            'body': json.dumps({'error': str(e)})
        }
