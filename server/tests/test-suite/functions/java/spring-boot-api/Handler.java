import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.*;

/**
 * Spring Boot-style REST API Handler
 * Use case: Enterprise-grade Java microservices
 */
public class Handler {
    
    private static final ObjectMapper mapper = new ObjectMapper();
    
    // In-memory database simulation
    private static final Map<String, Product> products = new HashMap<String, Product>() {{
        put("1", new Product("1", "Server", 5000.0, true));
        put("2", new Product("2", "Workstation", 2000.0, true));
        put("3", new Product("3", "Laptop", 1200.0, true));
    }};
    
    public Map<String, Object> handler(Map<String, Object> event, Map<String, Object> context) {
        String method = (String) event.getOrDefault("method", "GET");
        String path = (String) event.getOrDefault("path", "/");
        Map<String, Object> body = (Map<String, Object>) event.getOrDefault("body", new HashMap<>());
        
        try {
            if ("GET".equals(method)) {
                if (path.equals("/products") || path.equals("/")) {
                    // List all products
                    return createResponse(200, products.values());
                } else if (path.startsWith("/products/")) {
                    // Get single product
                    String productId = extractId(path);
                    Product product = products.get(productId);
                    
                    if (product != null) {
                        return createResponse(200, product);
                    } else {
                        return createErrorResponse(404, "Product not found");
                    }
                }
            } else if ("POST".equals(method) && path.equals("/products")) {
                // Create new product
                if (!body.containsKey("name") || !body.containsKey("price")) {
                    return createErrorResponse(400, "Missing required fields: name, price");
                }
                
                String newId = String.valueOf(products.size() + 1);
                Product newProduct = new Product(
                    newId,
                    (String) body.get("name"),
                    ((Number) body.get("price")).doubleValue(),
                    (Boolean) body.getOrDefault("available", true)
                );
                
                products.put(newId, newProduct);
                return createResponse(201, newProduct);
                
            } else if ("PUT".equals(method) && path.startsWith("/products/")) {
                // Update product
                String productId = extractId(path);
                Product product = products.get(productId);
                
                if (product == null) {
                    return createErrorResponse(404, "Product not found");
                }
                
                if (body.containsKey("name")) {
                    product.setName((String) body.get("name"));
                }
                if (body.containsKey("price")) {
                    product.setPrice(((Number) body.get("price")).doubleValue());
                }
                if (body.containsKey("available")) {
                    product.setAvailable((Boolean) body.get("available"));
                }
                
                return createResponse(200, product);
                
            } else if ("DELETE".equals(method) && path.startsWith("/products/")) {
                // Delete product
                String productId = extractId(path);
                
                if (products.containsKey(productId)) {
                    products.remove(productId);
                    return createResponse(204, Map.of("message", "Product deleted"));
                } else {
                    return createErrorResponse(404, "Product not found");
                }
            }
            
            return createErrorResponse(405, "Method not allowed");
            
        } catch (Exception e) {
            return createErrorResponse(500, "Internal server error: " + e.getMessage());
        }
    }
    
    private String extractId(String path) {
        String[] parts = path.split("/");
        return parts[parts.length - 1];
    }
    
    private Map<String, Object> createResponse(int statusCode, Object body) {
        Map<String, Object> response = new HashMap<>();
        response.put("statusCode", statusCode);
        response.put("headers", Map.of("Content-Type", "application/json"));
        
        try {
            response.put("body", mapper.writeValueAsString(body));
        } catch (Exception e) {
            response.put("body", "{}");
        }
        
        return response;
    }
    
    private Map<String, Object> createErrorResponse(int statusCode, String message) {
        return createResponse(statusCode, Map.of("error", message));
    }
    
    // Product POJO
    static class Product {
        private String id;
        private String name;
        private double price;
        private boolean available;
        
        public Product(String id, String name, double price, boolean available) {
            this.id = id;
            this.name = name;
            this.price = price;
            this.available = available;
        }
        
        // Getters and setters
        public String getId() { return id; }
        public void setId(String id) { this.id = id; }
        
        public String getName() { return name; }
        public void setName(String name) { this.name = name; }
        
        public double getPrice() { return price; }
        public void setPrice(double price) { this.price = price; }
        
        public boolean isAvailable() { return available; }
        public void setAvailable(boolean available) { this.available = available; }
    }
}
