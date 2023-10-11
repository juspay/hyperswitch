export REGION=us-east-2

#############  APPLICATION ##################
# CREATE SECURITY GROUP FOR APPLICATION

export EC2_SG="application-sg"

export APP_SG_ID=$(aws ec2 create-security-group \
--region $REGION \
--group-name $EC2_SG \
--description "Security Group for Hyperswitch EC2 instance" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--query 'GroupId' \
--output text \
)

echo `aws ec2 authorize-security-group-ingress \
--group-id $APP_SG_ID \
--protocol tcp \
--port 8080 \
--cidr 0.0.0.0/0 \
--region $REGION`

echo `aws ec2 authorize-security-group-ingress \
--group-id $APP_SG_ID \
--protocol tcp \
--port 22 \
--cidr 0.0.0.0/0 \
--region $REGION`

#############  REDIS ##################
# CREATE SECURITY GROUP FOR ELASTICACHE
export REDIS_GROUP_NAME=redis-sg
echo `aws ec2 create-security-group \
--group-name $REDIS_GROUP_NAME \
--description "SG attached to elasticache" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--region $REGION`


export REDIS_SG_ID=$(aws ec2 describe-security-groups --group-names $REDIS_GROUP_NAME --region $REGION --output text --query 'SecurityGroups[0].GroupId')

# CREATE INBOUND RULES
echo `aws ec2 authorize-security-group-ingress \
--group-id $REDIS_SG_ID \
--protocol tcp \
--port 6379 \
--source-group $EC2_SG \
--region $REGION`


#############  DB ##################

# CREATE SECURITY GROUP FOR RDS
export RDS_GROUP_NAME=rds-sg
echo `aws ec2 create-security-group \
--group-name $RDS_GROUP_NAME \
--description "SG attached to RDS" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--region $REGION`

export RDS_SG_ID=$(aws ec2 describe-security-groups --group-names $RDS_GROUP_NAME --region $REGION --output text --query 'SecurityGroups[0].GroupId')

# CREATE INBOUND RULES
echo `aws ec2 authorize-security-group-ingress \
--group-id $RDS_SG_ID \
--protocol tcp \
--port 5432 \
--source-group $EC2_SG \
--region $REGION`

echo `aws ec2 authorize-security-group-ingress \
    --group-id $RDS_SG_ID \
    --protocol tcp \
    --port 5432 \
    --cidr 0.0.0.0/0 \
    --region $REGION`

# CREATE ELASTICACHE WITH REDIS ENGINE
export CACHE_CLUSTER_ID=hyperswitch-cluster

echo `aws elasticache create-cache-cluster \
--cache-cluster-id $CACHE_CLUSTER_ID \
--cache-node-type cache.t3.medium \
--engine redis \
--num-cache-nodes 1 \
--security-group-ids $REDIS_SG_ID \
--engine-version 7.0 \
--tags "Key=ManagedBy,Value=hyperswitch" \
--region $REGION`

# CREATE RDS WITH PSQL
export DB_INSTANCE_ID=hyperswitch-db
echo `aws rds create-db-instance  \
    --db-instance-identifier $DB_INSTANCE_ID\
    --db-instance-class db.t3.micro \
    --engine postgres \
    --allocated-storage 20 \
    --master-username hyperswitch \
    --master-user-password hyps1234 \
    --backup-retention-period 7 \
    --region $REGION \
    --db-name hyperswitch_db \
    --tags "Key=ManagedBy,Value=hyperswitch" \
    --vpc-security-group-ids $RDS_SG_ID`
curl https://raw.githubusercontent.com/juspay/hyperswitch/feat/create-prod-script/schema.sql >> schema.sql

while [[ $RDS_STATUS != 'available' ]]; do
	echo $RDS_STATUS
	sleep 10

export RDS_STATUS=$(aws rds describe-db-instances \
--db-instance-identifier $DB_INSTANCE_ID \
--region $REGION \
--query "DBInstances[0].DBInstanceStatus" \
--output text)
done

export RDS_ENDPOINT=$(aws rds describe-db-instances --db-instance-identifier $DB_INSTANCE_ID --region $REGION --query "DBInstances[*].Endpoint.Address" --output text)
psql -d postgresql://hyperswitch:hyps1234@$RDS_ENDPOINT/hyperswitch_db -a -f schema.sql > /dev/null

cat << EOF > .user_data.sh
#!/bin/bash

sudo yum update -y
sudo amazon-linux-extras install docker
sudo service docker start
sudo usermod -a -G docker ec2-user

docker pull juspaydotin/hyperswitch-router:beta

curl https://raw.githubusercontent.com/juspay/hyperswitch/v1.55.0/config/development.toml > production.toml
EOF

export redis_status=$(aws elasticache describe-cache-clusters \
  --region $REGION \
  --cache-cluster-id $CACHE_CLUSTER_ID \
  --query 'CacheClusters[0].CacheClusterStatus' \
  --output text)

while [ $redis_status -ne 'available' ]
do
  echo "$redis_status"
  sleep 10
  export redis_status=$(aws elasticache describe-cache-clusters \
        --region $REGION \
        --cache-cluster-id $CACHE_CLUSTER_ID \
        --query 'CacheClusters[0].CacheClusterStatus' \
         --output text)
done

export REDIS_ENDPOINT=$(aws elasticache describe-cache-clusters \
    --region $REGION \
    --cache-cluster-id $CACHE_CLUSTER_ID \
    --show-cache-node-info \
    --query 'CacheClusters[0].CacheNodes[].Endpoint.Address' \
    --output text)

export RDS_STATUS=$(aws rds describe-db-instances \
--db-instance-identifier $DB_INSTANCE_ID \
--region $REGION \
--query "DBInstances[0].DBInstanceStatus" \
--output text)

while [ $RDS_STATUS -ne 'available' ]; do
	echo $RDS_STATUS
	sleep 10
	
export RDS_STATUS=$(aws rds describe-db-instances \
--db-instance-identifier $DB_INSTANCE_ID \
--region $REGION \
--query "DBInstances[0].DBInstanceStatus" \
--output text)
done

export RDS_ENDPOINT=$(aws rds describe-db-instances \
--db-instance-identifier $DB_INSTANCE_ID \
--region $REGION \
--query "DBInstances[*].Endpoint.Address" \
--output text)


echo "\n# Add redis and DB configs\n" >> user_data.sh
echo "cat << EOF >> .env" >> user_data.sh
echo "ROUTER__REDIS__CLUSTER_URLS=$REDIS_ENDPOINT" >> user_data.sh 
echo "ROUTER__MASTER_DATABASE__HOST=$RDS_ENDPOINT" >> user_data.sh
echo "ROUTER__REPLICA_DATABASE__HOST=$RDS_ENDPOINT" >> user_data.sh
echo "EOF" >> user_data.sh


docker run --env-file .env -p 8080:8080 -v `pwd`/:/local/config juspaydotin/hyperswitch-router:beta ./router -f /local/config/production.toml
" >> user_data.sh


export AWS_AMI_ID=$(aws ec2 describe-images --owners amazon --filters "Name=name,Values=amzn2-ami-hvm-2.0.*" --query 'sort_by(Images, &CreationDate)[-1].ImageId' --output text --region $REGION)

aws ec2 create-key-pair \
    --key-name hyperswitch-ec2-keypair \
    --query 'KeyMaterial' \
  --tag-specifications "ResourceType=key-pair,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
  --region $REGION \
    --output text > hyperswitch-keypair.pem
  

chmod 400 hyperswitch-keypair.pem

export HYPERSWITCH_INSTANCE_ID=$(aws ec2 run-instances \
    --image-id $AWS_AMI_ID \
    --instance-type t3.medium \
    --key-name hyperswitch-ec2-keypair \
    --monitoring "Enabled=false" \
    --security-group-ids $APP_SG_ID \
    --user-data file://./user_data.sh \
    --query 'Instances[0].InstanceId' \
    --output text \
  --region $REGION)

echo `aws ec2 create-tags \
--resources $HYPERSWITCH_INSTANCE_ID \
--tags "Key=Name,Value=hyperswitch-router" \
--region $REGION`

echo `aws ec2 create-tags \
--resources $HYPERSWITCH_INSTANCE_ID \
--tags "Key=ManagedBy,Value=hyperswitch" \
--region $REGION`

export PUBLIC_HYPERSWITCH_IP=$(aws ec2 describe-instances \
--instance-ids $HYPERSWITCH_INSTANCE_ID \
--query "Reservations[*].Instances[*].PublicIpAddress" \
--output=text \
--region $REGION)

echo $PUBLIC_HYPERSWITCH_IP
