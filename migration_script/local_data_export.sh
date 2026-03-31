#!/bin/bash

# === CONFIG ===
DB_NAME="hyperswitch_db"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/local_data.csv"

# === EXECUTE SQL ===
psql "$DB_NAME" -c "\copy (SELECT 
  ROW_NUMBER() OVER (ORDER BY ra.profile_id) AS serial_id,
  ra.profile_id,
  bp.merchant_id,
  bp.routing_algorithm ->> 'algorithm_id' AS active_algorithm_id,
  JSON_AGG(alg.algorithm_id ORDER BY alg.created_at) AS all_algorithm_ids
FROM (
  SELECT DISTINCT profile_id
  FROM routing_algorithm
  WHERE algorithm_for = 'payment'
) AS ra
JOIN business_profile bp
  ON ra.profile_id = bp.profile_id
JOIN routing_algorithm alg
  ON ra.profile_id = alg.profile_id AND alg.algorithm_for = 'payment'
WHERE bp.routing_algorithm ->> 'algorithm_id' IS NOT NULL
GROUP BY ra.profile_id, bp.merchant_id, bp.routing_algorithm ->> 'algorithm_id'
ORDER BY ra.profile_id) TO '$OUTPUT_FILE' WITH CSV HEADER;"

echo "✅ Exported to: $OUTPUT_FILE"
