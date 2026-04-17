import java.util.*;
import java.util.stream.Collectors;

/**
 * Batch Processing Handler
 * Use case: High-volume data processing with parallel execution
 */
public class Handler {
    
    public Map<String, Object> handler(Map<String, Object> event, Map<String, Object> context) {
        String operation = (String) event.getOrDefault("operation", "process");
        List<Map<String, Object>> batch = (List<Map<String, Object>>) 
            event.getOrDefault("batch", new ArrayList<>());
        Map<String, Object> config = (Map<String, Object>) 
            event.getOrDefault("config", new HashMap<>());
        
        try {
            switch (operation) {
                case "process":
                    return processBatch(batch, config);
                case "validate":
                    return validateBatch(batch, config);
                case "transform":
                    return transformBatch(batch, config);
                case "aggregate":
                    return aggregateBatch(batch, config);
                default:
                    return createErrorResponse(400, "Unknown operation: " + operation);
            }
        } catch (Exception e) {
            return createErrorResponse(500, "Processing error: " + e.getMessage());
        }
    }
    
    private Map<String, Object> processBatch(List<Map<String, Object>> batch, 
                                              Map<String, Object> config) {
        long startTime = System.currentTimeMillis();
        
        List<Map<String, Object>> processed = new ArrayList<>();
        List<Map<String, Object>> failed = new ArrayList<>();
        
        for (Map<String, Object> item : batch) {
            try {
                // Simulate processing
                Map<String, Object> processedItem = new HashMap<>(item);
                processedItem.put("processed_at", new Date().toString());
                processedItem.put("status", "completed");
                
                // Apply business logic
                if (item.containsKey("value")) {
                    Object value = item.get("value");
                    if (value instanceof Number) {
                        double numValue = ((Number) value).doubleValue();
                        processedItem.put("squared", numValue * numValue);
                        processedItem.put("doubled", numValue * 2);
                    }
                }
                
                processed.add(processedItem);
            } catch (Exception e) {
                Map<String, Object> errorItem = new HashMap<>(item);
                errorItem.put("error", e.getMessage());
                failed.add(errorItem);
            }
        }
        
        long duration = System.currentTimeMillis() - startTime;
        
        Map<String, Object> result = new HashMap<>();
        result.put("processed", processed);
        result.put("failed", failed);
        result.put("statistics", Map.of(
            "total", batch.size(),
            "successful", processed.size(),
            "failed", failed.size(),
            "duration_ms", duration,
            "throughput", batch.size() * 1000.0 / duration
        ));
        
        return createResponse(200, result);
    }
    
    private Map<String, Object> validateBatch(List<Map<String, Object>> batch,
                                               Map<String, Object> config) {
        List<String> requiredFields = (List<String>) 
            config.getOrDefault("required_fields", new ArrayList<>());
        
        List<Map<String, Object>> valid = new ArrayList<>();
        List<Map<String, Object>> invalid = new ArrayList<>();
        
        for (Map<String, Object> item : batch) {
            List<String> missingFields = new ArrayList<>();
            
            for (String field : requiredFields) {
                if (!item.containsKey(field) || item.get(field) == null) {
                    missingFields.add(field);
                }
            }
            
            if (missingFields.isEmpty()) {
                valid.add(item);
            } else {
                Map<String, Object> invalidItem = new HashMap<>(item);
                invalidItem.put("missing_fields", missingFields);
                invalid.add(invalidItem);
            }
        }
        
        Map<String, Object> result = new HashMap<>();
        result.put("valid", valid);
        result.put("invalid", invalid);
        result.put("validation_rate", (double) valid.size() / batch.size() * 100);
        
        return createResponse(200, result);
    }
    
    private Map<String, Object> transformBatch(List<Map<String, Object>> batch,
                                                Map<String, Object> config) {
        String transformType = (String) config.getOrDefault("type", "uppercase");
        
        List<Map<String, Object>> transformed = batch.stream()
            .map(item -> {
                Map<String, Object> newItem = new HashMap<>(item);
                
                if ("uppercase".equals(transformType)) {
                    item.forEach((key, value) -> {
                        if (value instanceof String) {
                            newItem.put(key, ((String) value).toUpperCase());
                        }
                    });
                } else if ("lowercase".equals(transformType)) {
                    item.forEach((key, value) -> {
                        if (value instanceof String) {
                            newItem.put(key, ((String) value).toLowerCase());
                        }
                    });
                }
                
                return newItem;
            })
            .collect(Collectors.toList());
        
        return createResponse(200, Map.of("transformed", transformed));
    }
    
    private Map<String, Object> aggregateBatch(List<Map<String, Object>> batch,
                                                Map<String, Object> config) {
        String groupBy = (String) config.getOrDefault("group_by", "type");
        
        Map<String, List<Map<String, Object>>> groups = batch.stream()
            .collect(Collectors.groupingBy(
                item -> String.valueOf(item.getOrDefault(groupBy, "unknown"))
            ));
        
        Map<String, Map<String, Object>> aggregates = new HashMap<>();
        
        for (Map.Entry<String, List<Map<String, Object>>> entry : groups.entrySet()) {
            String key = entry.getKey();
            List<Map<String, Object>> items = entry.getValue();
            
            double sum = items.stream()
                .filter(item -> item.containsKey("value"))
                .mapToDouble(item -> ((Number) item.get("value")).doubleValue())
                .sum();
            
            aggregates.put(key, Map.of(
                "count", items.size(),
                "sum", sum,
                "average", items.size() > 0 ? sum / items.size() : 0
            ));
        }
        
        return createResponse(200, Map.of(
            "aggregates", aggregates,
            "total_groups", groups.size()
        ));
    }
    
    private Map<String, Object> createResponse(int statusCode, Object body) {
        Map<String, Object> response = new HashMap<>();
        response.put("statusCode", statusCode);
        response.put("body", body);
        return response;
    }
    
    private Map<String, Object> createErrorResponse(int statusCode, String message) {
        return createResponse(statusCode, Map.of("error", message));
    }
}
