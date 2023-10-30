#!/bin/bash

command_discovery() {
  type $1 > /dev/null 2> /dev/null
  if [[ $? != 0 ]]; then
    echo "\`$1\` command not found"
    exit 1
  fi
}

command_discovery curl
command_discovery aws
command_discovery psql

echo "Please enter the AWS region (us-east-2): "
read REGION < /dev/tty

if [ -z "$REGION" ]; then
    echo "Using default region: us-east-2"
    REGION="us-east-2"
fi

while [[ -z "$MASTER_DB_PASSWORD" ]]; do
    echo "Please enter the password for your RDS instance: "
    echo "Minimum length: 8 Characters [A-Z][a-z][0-9]"
    read MASTER_DB_PASSWORD < /dev/tty
done

while [[ -z "$ADMIN_API_KEY" ]]; do
    echo "Please configure the Admin api key: (Required to access Hyperswitch APIs)"
    read ADMIN_API_KEY < /dev/tty
done

#############  APPLICATION ##################
# CREATE SECURITY GROUP FOR APPLICATION

echo "Creating Security Group for Application..."

export EC2_SG="application-sg"

echo `(aws ec2 create-security-group \
--region $REGION \
--group-name $EC2_SG \
--description "Security Group for Hyperswitch EC2 instance" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
)`

export APP_SG_ID=$(aws ec2 describe-security-groups --group-names $EC2_SG --region $REGION --output text --query 'SecurityGroups[0].GroupId')

echo "Security Group for Application CREATED.\n"

echo "Creating Security Group ingress for port 80..."

echo `aws ec2 authorize-security-group-ingress \
--group-id $APP_SG_ID \
--protocol tcp \
--port 80 \
--cidr 0.0.0.0/0 \
--region $REGION`

echo "Security Group ingress for port 80 CREATED.\n"


echo "Creating Security Group ingress for port 22..."

echo `aws ec2 authorize-security-group-ingress \
--group-id $APP_SG_ID \
--protocol tcp \
--port 22 \
--cidr 0.0.0.0/0 \
--region $REGION`

echo "Security Group ingress for port 22 CREATED.\n"

#############  REDIS ##################
# CREATE SECURITY GROUP FOR ELASTICACHE

echo "Creating Security Group for Elasticache..."

export REDIS_GROUP_NAME=redis-sg
echo `aws ec2 create-security-group \
--group-name $REDIS_GROUP_NAME \
--description "SG attached to elasticache" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--region $REGION`

echo "Security Group for Elasticache CREATED.\n"

echo "Creating Inbound rules for Redis..."

export REDIS_SG_ID=$(aws ec2 describe-security-groups --group-names $REDIS_GROUP_NAME --region $REGION --output text --query 'SecurityGroups[0].GroupId')

# CREATE INBOUND RULES
echo `aws ec2 authorize-security-group-ingress \
--group-id $REDIS_SG_ID \
--protocol tcp \
--port 6379 \
--source-group $EC2_SG \
--region $REGION`

echo "Inbound rules for Redis CREATED.\n"

#############  DB ##################

echo "Creating Security Group for RDS..."

export RDS_GROUP_NAME=rds-sg
echo `aws ec2 create-security-group \
--group-name $RDS_GROUP_NAME \
--description "SG attached to RDS" \
--tag-specifications "ResourceType=security-group,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
--region $REGION`

echo "Security Group for RDS CREATED.\n"

echo "Creating Inbound rules for RDS..."

export RDS_SG_ID=$(aws ec2 describe-security-groups --group-names $RDS_GROUP_NAME --region $REGION --output text --query 'SecurityGroups[0].GroupId')

# CREATE INBOUND RULES
echo `aws ec2 authorize-security-group-ingress \
--group-id $RDS_SG_ID \
--protocol tcp \
--port 5432 \
--source-group $EC2_SG \
--region $REGION`

echo "Inbound rules for RDS CREATED.\n"

echo `aws ec2 authorize-security-group-ingress \
    --group-id $RDS_SG_ID \
    --protocol tcp \
    --port 5432 \
    --cidr 0.0.0.0/0 \
    --region $REGION`

echo "Inbound rules for RDS (from any IP) CREATED.\n"

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

echo "Elasticache with Redis engine CREATED.\n"

echo "Creating RDS with PSQL..."

export DB_INSTANCE_ID=hyperswitch-db
echo `aws rds create-db-instance  \
    --db-instance-identifier $DB_INSTANCE_ID\
    --db-instance-class db.t3.micro \
    --engine postgres \
    --allocated-storage 20 \
    --master-username hyperswitch \
    --master-user-password $MASTER_DB_PASSWORD \
    --backup-retention-period 7 \
    --region $REGION \
    --db-name hyperswitch_db \
    --tags "Key=ManagedBy,Value=hyperswitch" \
    --vpc-security-group-ids $RDS_SG_ID`

echo "RDS with PSQL CREATED.\n"

echo "Downloading Hyperswitch PSQL Schema..."

curl https://raw.githubusercontent.com/juspay/hyperswitch/main/aws/beta_schema.sql > schema.sql

echo "Schema.sql downloaded.\n"

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

echo "RDS Initialized.\n"

echo "Retrieving RDS Endpoint..."

export RDS_ENDPOINT=$(aws rds describe-db-instances --db-instance-identifier $DB_INSTANCE_ID --region $REGION --query "DBInstances[0].Endpoint.Address" --output text)

echo "RDS Endpoint retrieved.\n"

echo "Applying Schema to DB..."

psql -d postgresql://hyperswitch:$MASTER_DB_PASSWORD@$RDS_ENDPOINT/hyperswitch_db -a -f schema.sql > /dev/null

echo "Schema applied to DB.\n"

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

echo "Redis Initialized.\n"

echo "Retrieving Redis Endpoint..."

export REDIS_ENDPOINT=$(aws elasticache describe-cache-clusters \
    --region $REGION \
    --cache-cluster-id $CACHE_CLUSTER_ID \
    --show-cache-node-info \
    --query 'CacheClusters[0].CacheNodes[].Endpoint.Address' \
    --output text)

echo "Redis Endpoint retrieved.\n"

echo "\n# Add redis and DB configs.\n" >> user_data.sh
echo "cat << EOF >> .env" >> user_data.sh
echo "ROUTER__REDIS__HOST=$REDIS_ENDPOINT" >> user_data.sh
echo "ROUTER__MASTER_DATABASE__HOST=$RDS_ENDPOINT" >> user_data.sh
echo "ROUTER__REPLICA_DATABASE__HOST=$RDS_ENDPOINT" >> user_data.sh
echo "ROUTER__SERVER__HOST=0.0.0.0" >> user_data.sh
echo "ROUTER__MASTER_DATABASE__USERNAME=hyperswitch" >> user_data.sh
echo "ROUTER__MASTER_DATABASE__PASSWORD=$MASTER_DB_PASSWORD" >> user_data.sh
echo "ROUTER__SERVER__BASE_URL=\$(curl ifconfig.me)" >> user_data.sh
echo "ROUTER__SECRETS__ADMIN_API_KEY=$ADMIN_API_KEY" >> user_data.sh
echo "EOF" >> user_data.sh

echo "docker run --env-file .env -p 80:8080 -v \`pwd\`/:/local/config juspaydotin/hyperswitch-router:beta ./router -f /local/config/production.toml
" >> user_data.sh

echo "Retrieving AWS AMI ID..."

export AWS_AMI_ID=$(aws ec2 describe-images --owners amazon --filters "Name=name,Values=amzn2-ami-hvm-2.0.*" --query 'sort_by(Images, &CreationDate)[-1].ImageId' --output text --region $REGION)

echo "AWS AMI ID retrieved.\n"

echo "Creating EC2 Keypair..."

rm -rf hyperswitch-keypair.pem

aws ec2 create-key-pair \
    --key-name hyperswitch-ec2-keypair \
    --query 'KeyMaterial' \
    --tag-specifications "ResourceType=key-pair,Tags=[{Key=ManagedBy,Value=hyperswitch}]" \
    --region $REGION \
    --output text > hyperswitch-keypair.pem

echo "Keypair created and saved to hyperswitch-keypair.pem.\n"

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

echo "EC2 instance launched.\n"

echo "Add Tags to EC2 instance..."

echo `aws ec2 create-tags \
--resources $HYPERSWITCH_INSTANCE_ID \
--tags "Key=Name,Value=hyperswitch-router" \
--region $REGION`

echo "Tag added to EC2 instance.\n"

echo `aws ec2 create-tags \
--resources $HYPERSWITCH_INSTANCE_ID \
--tags "Key=ManagedBy,Value=hyperswitch" \
--region $REGION`

echo "ManagedBy tag added to EC2 instance.\n"

echo "Retrieving the Public IP of Hyperswitch EC2 Instance..."
export PUBLIC_HYPERSWITCH_IP=$(aws ec2 describe-instances \
--instance-ids $HYPERSWITCH_INSTANCE_ID \
--query "Reservations[*].Instances[*].PublicIpAddress" \
--output=text \
--region $REGION)

health_status=null
while [[ $health_status != 'health is good' ]]
do
    health_status=$(curl http://$PUBLIC_HYPERSWITCH_IP/health)
    sleep 10
done

echo "Hurray! You can try using hyperswitch at http://$PUBLIC_HYPERSWITCH_IP"
echo "Health endpoint: http://$PUBLIC_HYPERSWITCH_IP/health"
