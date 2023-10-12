#!/bin/bash

export REGION=us-east-2


command_discovery() {
  type $1 > /dev/null 2> /dev/null
  if [[ $? != 0 ]]; then
    echo "\`$1\` command not found"
    exit 1
  fi
}

yes_or_no() {
    read response < /dev/tty
    case $response in
        [Yy]* ) return 0 ;;
        [Nn]* ) return 1 ;;
        * ) return 1 ;;
    esac
}

command_discovery aws
command_discovery jq


export ALL_ELASTIC_CACHE=($(aws elasticache describe-cache-clusters \
  --region $REGION \
    --query "CacheClusters[*].ARN" --output text))

for cluster_id in $ALL_ELASTIC_CACHE; do
    cluster_id=${cluster_arn##*:}

    aws elasticache list-tags-for-resource \
        --resource-name $cluster_id \
    --region $REGION \
        --output json | jq \
        '.TagList[] | select( [ .Key == "ManagedBy" and .Value == "hyperswitch" ] | any)' \
        -e > /dev/null

    if [[ $? -eq 0 ]]; then
        echo -n "Delete $cluster_id (Y/n)? "
        if yes_or_no; then
            aws elasticache delete-cache-cluster --region $REGION --cache-cluster-id $cluster_id
        fi
    fi
done

export ALL_KEY_PAIRS=($(aws ec2 describe-key-pairs \
            --filters "Name=tag:ManagedBy,Values=hyperswitch" \
--region $REGION \
    --query 'KeyPairs[*].KeyPairId' --output text))

echo -n "Deleting ( $ALL_KEY_PAIRS ) key pairs? (Y/n)?"

if yes_or_no; then
    for KEY_ID in $ALL_KEY_PAIRS; do
        aws ec2 delete-key-pair --key-pair-id $KEY_ID --region $REGION
    done
fi

export ALL_INSTANCES=($(aws ec2 describe-instances \
            --filters 'Name=tag:ManagedBy,Values=hyperswitch' \
--region $REGION \
    --query 'Reservations[*].Instances[*].InstanceId' --output text))

export ALL_EBS=($(aws ec2 describe-instances \
            --filters 'Name=tag:ManagedBy,Values=hyperswitch' \
--region $REGION \
            --query 'Reservations[*].Instances[*].BlockDeviceMappings[*].Ebs.VolumeId' \
    --output text))

echo -n "Terminating ( $ALL_INSTANCES ) instances? (Y/n)?"

if yes_or_no; then
    for INSTANCE_ID in $ALL_INSTANCES; do
        aws ec2 terminate-instances --instance-ids $INSTANCE_ID --region $REGION
    done
fi

export ALL_DB_RESOURCES=($(aws rds describe-db-instances \
--region $REGION \
    --query 'DBInstances[*].DBInstanceArn' --output text))

for resource_id in $ALL_DB_RESOURCES; do
    aws rds list-tags-for-resource \
        --resource-name $resource_id \
   --region $REGION \
        --output json | jq \
        '.TagList[] | select( [ .Key == "ManagedBy" and .Value == "hyperswitch" ] | any )' \
        -e > /dev/null

    if [[ $? -eq 0 ]]; then
        echo -n "Delete $resource_id (Y/n)? "
        if yes_or_no; then
            export DB_INSTANCE_ID=$(aws rds describe-db-instances \
       --region $REGION \
                --filters "Name=db-instance-id,Values=$resource_id" \
                --query 'DBInstances[*].DBInstanceIdentifier' --output text)


            echo "Create a snapshot before deleting ( $DB_INSTANCE_ID ) the database (Y/n)? "
            if yes_or_no; then
                cd aws rds delete-db-instance \
                    --db-instance-identifier $DB_INSTANCE_ID \
   --region $REGION \
                    --final-db-snapshot-identifier hyperswitch-db-snapshot-`date +%s`
            else
                aws rds delete-db-instance \
         --region $REGION \
                    --db-instance-identifier $DB_INSTANCE_ID \
                    --skip-final-snapshot
            fi
        fi
    fi
done


export ALL_SECURITY_GROUPS=($(aws ec2 describe-security-groups \
            --filters 'Name=tag:ManagedBy,Values=hyperswitch' \
--region $REGION \
    --query 'SecurityGroups[*].GroupId' --output text))

echo -n "Deleting ( $ALL_SECURITY_GROUPS ) security groups? (Y/n)?"

if yes_or_no; then
    for GROUP_ID in $ALL_SECURITY_GROUPS; do
        aws ec2 delete-security-group --group-id $GROUP_ID --region $REGION
    done
fi
