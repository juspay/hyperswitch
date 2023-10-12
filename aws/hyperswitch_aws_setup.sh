export REGION=us-east-2

#############  APPLICATION ##################
# CREATE SECURITY GROUP FOR APPLICATION

echo "Creating Security Group for Application..."

export EC2_SG="application-sg"

export APP_SG_ID=$(aws ec2 create-security-group \
--region $REGION \
--group-name $EC2_SG \
--description "Security Group for Hyperswitch EC2 instance" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--query 'GroupId' \
--output text \
)

if [ $? -eq 0 ]; then
    echo "Security Group for Application CREATED successfully!"
else
    echo "Security Group for Application CREATION failed!"
    exit 1
fi

echo "Creating Security Group ingress for port 8080..."

echo `aws ec2 authorize-security-group-ingress \
--group-id $APP_SG_ID \
--protocol tcp \
--port 8080 \
--cidr 0.0.0.0/0 \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Security Group ingress for port 8080 SUCCESS!"
else
    echo "Security Group ingress for port 8080 FAILED!"
    exit 1
fi

echo "Creating Security Group ingress for port 22..."

echo `aws ec2 authorize-security-group-ingress \
--group-id $APP_SG_ID \
--protocol tcp \
--port 22 \
--cidr 0.0.0.0/0 \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Security Group ingress for port 22 SUCCESS!"
else
    echo "Security Group ingress for port 22 FAILED!"
    exit 1
fi

#############  REDIS ##################
# CREATE SECURITY GROUP FOR ELASTICACHE

echo "Creating Security Group for Elasticache..."

export REDIS_GROUP_NAME=redis-sg
echo `aws ec2 create-security-group \
--group-name $REDIS_GROUP_NAME \
--description "SG attached to elasticache" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Security Group for Elasticache CREATED successfully!"
else
    echo "Security Group for Elasticache CREATION failed!"
    exit 1
fi

echo "Creating Inbound rules for Redis..."

export REDIS_SG_ID=$(aws ec2 describe-security-groups --group-names $REDIS_GROUP_NAME --region $REGION --output text --query 'SecurityGroups[0].GroupId')

# CREATE INBOUND RULES
echo `aws ec2 authorize-security-group-ingress \
--group-id $REDIS_SG_ID \
--protocol tcp \
--port 6379 \
--source-group $EC2_SG \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Inbound rules for Redis CREATED successfully!"
else
    echo "Inbound rules for Redis CREATION failed!"
    exit 1
fi

#############  DB ##################

echo "Creating Security Group for RDS..."

export RDS_GROUP_NAME=rds-sg
echo `aws ec2 create-security-group \
--group-name $RDS_GROUP_NAME \
--description "SG attached to RDS" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Security Group for RDS CREATED successfully!"
else
    echo "Security Group for RDS CREATION failed!"
    exit 1
fi

echo "Creating Inbound rules for RDS..."

export RDS_SG_ID=$(aws ec2 describe-security-groups --group-names $RDS_GROUP_NAME --region $REGION --output text --query 'SecurityGroups[0].GroupId')

# CREATE INBOUND RULES
echo `aws ec2 authorize-security-group-ingress \
--group-id $RDS_SG_ID \
--protocol tcp \
--port 5432 \
--source-group $EC2_SG \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Inbound rules for RDS CREATED successfully!"
else
    echo "Inbound rules for RDS CREATION failed!"
    exit 1
fi

echo `aws ec2 authorize-security-group-ingress \
    --group-id $RDS_SG_ID \
    --protocol tcp \
    --port 5432 \
    --cidr 0.0.0.0/0 \
    --region $REGION`

if [ $? -eq 0 ]; then
    echo "Inbound rules for RDS (from any IP) CREATED successfully!"
else
    echo "Inbound rules for RDS (from any IP) CREATION failed!"
    exit 1
fi

echo "Creating Elasticache with Redis engine..."

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

if [ $? -eq 0 ]; then
    echo "Elasticache with Redis engine CREATED successfully!"
else
    echo "Elasticache with Redis engine CREATION failed!"
    exit 1
fi

echo "Creating RDS with PSQL..."

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

if [ $? -eq 0 ]; then
    echo "RDS with PSQL CREATED successfully!"
else
    echo "RDS with PSQL CREATION failed!"
    exit 1
fi

echo "Downloading Hyperswitch PSQL Schema..."

curl https://raw.githubusercontent.com/juspay/hyperswitch/feat/create-prod-script/aws/schema.sql > schema.sql

if [ $? -eq 0 ]; then
    echo "Schema.sql downloaded successfully!"
else
    echo "Schema.sql download failed!"
    exit 1
fi

echo "Awaiting RDS Initialization..."

export RDS_STATUS=$(aws rds describe-db-instances \
--db-instance-identifier $DB_INSTANCE_ID \
--region $REGION \
--query "DBInstances[0].DBInstanceStatus" \
--output text)

while [[ $RDS_STATUS != 'available' ]]; do
    echo $RDS_STATUS
    sleep 10

    export RDS_STATUS=$(aws rds describe-db-instances \
    --db-instance-identifier $DB_INSTANCE_ID \
    --region $REGION \
    --query "DBInstances[0].DBInstanceStatus" \
    --output text)
done

echo "RDS Initialized successfully!"

echo "Retrieving RDS Endpoint..."

export RDS_ENDPOINT=$(aws rds describe-db-instances --db-instance-identifier $DB_INSTANCE_ID --region $REGION --query "DBInstances[0].Endpoint.Address" --output text)

if [ $? -eq 0 ]; then
    echo "RDS Endpoint retrieved successfully!"
else
    echo "RDS Endpoint retrieval failed!"
    exit 1
fi

echo "Applying Schema to DB..."

psql -d postgresql://hyperswitch:hyps1234@$RDS_ENDPOINT/hyperswitch_db -a -f schema.sql > /dev/null

if [ $? -eq 0 ]; then
    echo "Schema applied to DB successfully!"
else
    echo "Schema application to DB failed!"
    exit 1
fi

cat << EOF > user_data.sh
#!/bin/bash

sudo yum update -y
sudo amazon-linux-extras install docker
sudo service docker start
sudo usermod -a -G docker ec2-user

docker pull juspaydotin/hyperswitch-router:beta

curl https://raw.githubusercontent.com/juspay/hyperswitch/v1.55.0/config/development.toml > production.toml

EOF

echo "Awaiting Redis Initialization..."

export redis_status=$(aws elasticache describe-cache-clusters \
  --region $REGION \
  --cache-cluster-id $CACHE_CLUSTER_ID \
  --query 'CacheClusters[0].CacheClusterStatus' \
  --output text)

while [ $redis_status != 'available' ]
do
    echo "$redis_status"
    sleep 10
    export redis_status=$(aws elasticache describe-cache-clusters \
        --region $REGION \
        --cache-cluster-id $CACHE_CLUSTER_ID \
        --query 'CacheClusters[0].CacheClusterStatus' \
        --output text)
done

echo "Redis Initialized successfully!"

echo "Retrieving Redis Endpoint..."

export REDIS_ENDPOINT=$(aws elasticache describe-cache-clusters \
    --region $REGION \
    --cache-cluster-id $CACHE_CLUSTER_ID \
    --show-cache-node-info \
    --query 'CacheClusters[0].CacheNodes[].Endpoint.Address' \
    --output text)

if [ $? -eq 0 ]; then
    echo "Redis Endpoint retrieved successfully!"
else
    echo "Redis Endpoint retrieval failed!"
    exit 1
fi


echo "\n# Add redis and DB configs\n" >> user_data.sh
echo "cat << EOF >> .env" >> user_data.sh
echo "ROUTER__REDIS__HOST=$REDIS_ENDPOINT" >> user_data.sh
echo "ROUTER__MASTER_DATABASE__HOST=$RDS_ENDPOINT" >> user_data.sh
echo "ROUTER__REPLICA_DATABASE__HOST=$RDS_ENDPOINT" >> user_data.sh
echo "ROUTER__SERVER__HOST=0.0.0.0" >> user_data.sh
echo "EOF" >> user_data.sh

echo "docker run --env-file .env -p 8080:8080 -v \`pwd\`/:/local/config juspaydotin/hyperswitch-router:beta ./router -f /local/config/production.toml
" >> user_data.sh

echo "Retrieving AWS AMI ID..."

export AWS_AMI_ID=$(aws ec2 describe-images --owners amazon --filters "Name=name,Values=amzn2-ami-hvm-2.0.*" --query 'sort_by(Images, &CreationDate)[-1].ImageId' --output text --region $REGION)

if [ $? -eq 0 ]; then
    echo "AWS AMI ID retrieved successfully!"
else
    echo "AWS AMI ID retrieval failed!"
    exit 1
fi

echo "Creating EC2 Keypair..."

aws ec2 create-key-pair \
    --key-name hyperswitch-ec2-keypair \
    --query 'KeyMaterial' \
    --tag-specifications "ResourceType=key-pair,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
    --region $REGION \
    --output text > hyperswitch-keypair.pem

if [ $? -eq 0 ]; then
    echo "Keypair created and saved to hyperswitch-keypair.pem successfully!"
else
    echo "Keypair creation failed!"
    exit 1
fi

chmod 400 hyperswitch-keypair.pem

echo "Launching EC2 Instance..."

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

if [ $? -eq 0 ]; then
    echo "EC2 instance launched successfully!"
else
    echo "EC2 instance launch failed!"
    exit 1
fi

echo "Add Tags to EC2 instance..."

echo `aws ec2 create-tags \
--resources $HYPERSWITCH_INSTANCE_ID \
--tags "Key=Name,Value=hyperswitch-router" \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "Tag added to EC2 instance successfully!"
else
    echo "Tag addition to EC2 instance failed!"
    exit 1
fi

echo `aws ec2 create-tags \
--resources $HYPERSWITCH_INSTANCE_ID \
--tags "Key=ManagedBy,Value=hyperswitch" \
--region $REGION`

if [ $? -eq 0 ]; then
    echo "ManagedBy tag added to EC2 instance successfully!"
else
    echo "ManagedBy tag addition to EC2 instance failed!"
    exit 1
fi

echo "Retrieving the Public IP of Hyperswitch EC2 Instance..."
export PUBLIC_HYPERSWITCH_IP=$(aws ec2 describe-instances \
--instance-ids $HYPERSWITCH_INSTANCE_ID \
--query "Reservations[*].Instances[*].PublicIpAddress" \
--output=text \
--region $REGION)

if [ $? -eq 0 ]; then
    echo "Hurray! Public IP of EC2 instance retrieved: $PUBLIC_HYPERSWITCH_IP"
else
    echo "Public IP retrieval of EC2 instance failed!"
    exit 1
fi
