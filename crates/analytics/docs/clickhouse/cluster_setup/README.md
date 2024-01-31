# Tutorial for set up clickhouse server


## Single server with docker


- Run server

```
docker run -d --name clickhouse-server -p 9000:9000 --ulimit nofile=262144:262144 yandex/clickhouse-server

```

- Run client

```
docker run -it --rm --link clickhouse-server:clickhouse-server yandex/clickhouse-client  --host clickhouse-server
```

Now you can see if it success setup or not.


## Setup Cluster


This part we will setup

- 1 cluster, with 3 shards
- Each shard has 2 replica server
- Use ReplicatedMergeTree & Distributed table to setup our table.


### Cluster

Let's see our docker-compose.yml first.

```
version: '3'

services:
    clickhouse-zookeeper:
        image: zookeeper
        ports:
            - "2181:2181"
            - "2182:2182"
        container_name: clickhouse-zookeeper
        hostname: clickhouse-zookeeper

    clickhouse-01:
        image: yandex/clickhouse-server
        hostname: clickhouse-01
        container_name: clickhouse-01
        ports:
            - 9001:9000
        volumes:
                - ./config/clickhouse_config.xml:/etc/clickhouse-server/config.xml
                - ./config/clickhouse_metrika.xml:/etc/clickhouse-server/metrika.xml
                - ./config/macros/macros-01.xml:/etc/clickhouse-server/config.d/macros.xml
                # - ./data/server-01:/var/lib/clickhouse
        ulimits:
            nofile:
                soft: 262144
                hard: 262144
        depends_on:
            - "clickhouse-zookeeper"

    clickhouse-02:
        image: yandex/clickhouse-server
        hostname: clickhouse-02
        container_name: clickhouse-02
        ports:
            - 9002:9000
        volumes:
                - ./config/clickhouse_config.xml:/etc/clickhouse-server/config.xml
                - ./config/clickhouse_metrika.xml:/etc/clickhouse-server/metrika.xml
                - ./config/macros/macros-02.xml:/etc/clickhouse-server/config.d/macros.xml
                # - ./data/server-02:/var/lib/clickhouse
        ulimits:
            nofile:
                soft: 262144
                hard: 262144
        depends_on:
            - "clickhouse-zookeeper"

    clickhouse-03:
        image: yandex/clickhouse-server
        hostname: clickhouse-03
        container_name: clickhouse-03
        ports:
            - 9003:9000
        volumes:
                - ./config/clickhouse_config.xml:/etc/clickhouse-server/config.xml
                - ./config/clickhouse_metrika.xml:/etc/clickhouse-server/metrika.xml
                - ./config/macros/macros-03.xml:/etc/clickhouse-server/config.d/macros.xml
                # - ./data/server-03:/var/lib/clickhouse
        ulimits:
            nofile:
                soft: 262144
                hard: 262144
        depends_on:
            - "clickhouse-zookeeper"

    clickhouse-04:
        image: yandex/clickhouse-server
        hostname: clickhouse-04
        container_name: clickhouse-04
        ports:
            - 9004:9000
        volumes:
                - ./config/clickhouse_config.xml:/etc/clickhouse-server/config.xml
                - ./config/clickhouse_metrika.xml:/etc/clickhouse-server/metrika.xml
                - ./config/macros/macros-04.xml:/etc/clickhouse-server/config.d/macros.xml
                # - ./data/server-04:/var/lib/clickhouse
        ulimits:
            nofile:
                soft: 262144
                hard: 262144
        depends_on:
            - "clickhouse-zookeeper"

    clickhouse-05:
        image: yandex/clickhouse-server
        hostname: clickhouse-05
        container_name: clickhouse-05
        ports:
            - 9005:9000
        volumes:
                - ./config/clickhouse_config.xml:/etc/clickhouse-server/config.xml
                - ./config/clickhouse_metrika.xml:/etc/clickhouse-server/metrika.xml
                - ./config/macros/macros-05.xml:/etc/clickhouse-server/config.d/macros.xml
                # - ./data/server-05:/var/lib/clickhouse
        ulimits:
            nofile:
                soft: 262144
                hard: 262144
        depends_on:
            - "clickhouse-zookeeper"

    clickhouse-06:
        image: yandex/clickhouse-server
        hostname: clickhouse-06
        container_name: clickhouse-06
        ports:
            - 9006:9000
        volumes:
                - ./config/clickhouse_config.xml:/etc/clickhouse-server/config.xml
                - ./config/clickhouse_metrika.xml:/etc/clickhouse-server/metrika.xml
                - ./config/macros/macros-06.xml:/etc/clickhouse-server/config.d/macros.xml
                # - ./data/server-06:/var/lib/clickhouse
        ulimits:
            nofile:
                soft: 262144
                hard: 262144
        depends_on:
            - "clickhouse-zookeeper"
networks:
    default:
        external:
            name: clickhouse-net
```


We have 6 clickhouse server container and one zookeeper container.


**To enable replication ZooKeeper is required. ClickHouse will take care of data consistency on all replicas and run restore procedure after failure automatically. It's recommended to deploy ZooKeeper cluster to separate servers.**

**ZooKeeper is not a requirement — in some simple cases you can duplicate the data by writing it into all the replicas from your application code. This approach is not recommended — in this case ClickHouse is not able to guarantee data consistency on all replicas. This remains the responsibility of your application.**


Let's see config file.

`./config/clickhouse_config.xml` is the default config file in docker, we copy it out and add this line

```
    <!-- If element has 'incl' attribute, then for it's value will be used corresponding substitution from another file.
         By default, path to file with substitutions is /etc/metrika.xml. It could be changed in config in 'include_from' element.
         Values for substitutions are specified in /yandex/name_of_substitution elements in that file.
      -->
    <include_from>/etc/clickhouse-server/metrika.xml</include_from>
```


So lets see `clickhouse_metrika.xml`

```
<yandex>
	<clickhouse_remote_servers>
		<cluster_1>
			<shard>
                                <weight>1</weight>
                                <internal_replication>true</internal_replication>
				<replica>
					<host>clickhouse-01</host>
					<port>9000</port>
				</replica>
				<replica>
					<host>clickhouse-06</host>
					<port>9000</port>
				</replica>
			</shard>
			<shard>
                                <weight>1</weight>
                                <internal_replication>true</internal_replication>
				<replica>
					<host>clickhouse-02</host>
					<port>9000</port>
				</replica>
				<replica>
					<host>clickhouse-03</host>
					<port>9000</port>
				</replica>
			</shard>
			<shard>
                                <weight>1</weight>
                                <internal_replication>true</internal_replication>

				<replica>
					<host>clickhouse-04</host>
					<port>9000</port>
				</replica>
				<replica>
					<host>clickhouse-05</host>
					<port>9000</port>
				</replica>
			</shard>
		</cluster_1>
	</clickhouse_remote_servers>
        <zookeeper-servers>
            <node index="1">
                <host>clickhouse-zookeeper</host>
                <port>2181</port>
            </node>
        </zookeeper-servers>
        <networks>
            <ip>::/0</ip>
        </networks>
        <clickhouse_compression>
            <case>
                <min_part_size>10000000000</min_part_size>
                <min_part_size_ratio>0.01</min_part_size_ratio>
                <method>lz4</method>
            </case>
        </clickhouse_compression>
</yandex>
```

and macros.xml, each instances has there own macros settings, like server 1: 

```
<yandex>
    <macros>
        <replica>clickhouse-01</replica>
        <shard>01</shard>
        <layer>01</layer>
    </macros>
</yandex>
```


**Make sure your macros settings is equal to remote server settings in metrika.xml**

So now you can start the server.

```
docker network create clickhouse-net
docker-compose up -d
```

Conn to server and see if the cluster settings fine;

```
docker run -it --rm --network="clickhouse-net" --link clickhouse-01:clickhouse-server yandex/clickhouse-client --host clickhouse-server
```

```sql
clickhouse-01 :) select * from system.clusters;

SELECT *
FROM system.clusters 

┌─cluster─────────────────────┬─shard_num─┬─shard_weight─┬─replica_num─┬─host_name─────┬─host_address─┬─port─┬─is_local─┬─user────┬─default_database─┐
│ cluster_1                   │         1 │            1 │           1 │ clickhouse-01 │ 172.21.0.4   │ 9000 │        1 │ default │                  │
│ cluster_1                   │         1 │            1 │           2 │ clickhouse-06 │ 172.21.0.5   │ 9000 │        1 │ default │                  │
│ cluster_1                   │         2 │            1 │           1 │ clickhouse-02 │ 172.21.0.8   │ 9000 │        0 │ default │                  │
│ cluster_1                   │         2 │            1 │           2 │ clickhouse-03 │ 172.21.0.6   │ 9000 │        0 │ default │                  │
│ cluster_1                   │         3 │            1 │           1 │ clickhouse-04 │ 172.21.0.7   │ 9000 │        0 │ default │                  │
│ cluster_1                   │         3 │            1 │           2 │ clickhouse-05 │ 172.21.0.3   │ 9000 │        0 │ default │                  │
│ test_shard_localhost        │         1 │            1 │           1 │ localhost     │ 127.0.0.1    │ 9000 │        1 │ default │                  │
│ test_shard_localhost_secure │         1 │            1 │           1 │ localhost     │ 127.0.0.1    │ 9440 │        0 │ default │                  │
└─────────────────────────────┴───────────┴──────────────┴─────────────┴───────────────┴──────────────┴──────┴──────────┴─────────┴──────────────────┘
```

If you see this, it means cluster's settings work well(but not conn fine).


### Replica Table

So now we have a cluster and replica settings. For clickhouse, we need to create ReplicatedMergeTree Table as a local table in every server.

```sql
CREATE TABLE ttt (id Int32) ENGINE = ReplicatedMergeTree('/clickhouse/tables/{layer}-{shard}/ttt', '{replica}') PARTITION BY id ORDER BY id
```

and Create Distributed Table conn to local table

```sql
CREATE TABLE ttt_all as ttt ENGINE = Distributed(cluster_1, default, ttt, rand());
```


### Insert and test

gen some data and test.


```
# docker exec into client server 1 and
for ((idx=1;idx<=100;++idx)); do clickhouse-client --host clickhouse-server --query "Insert into default.ttt_all values ($idx)"; done;
```

For Distributed table.

```
select count(*) from ttt_all;
```

For loacl table.

```
select count(*) from ttt;
```


## Authentication

Please see config/users.xml


- Conn
```bash
docker run -it --rm --network="clickhouse-net" --link clickhouse-01:clickhouse-server yandex/clickhouse-client --host clickhouse-server -u user1 --password 123456
```

## Source

- https://clickhouse.yandex/docs/en/operations/table_engines/replication/#creating-replicated-tables
