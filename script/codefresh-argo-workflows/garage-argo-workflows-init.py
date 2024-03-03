import garage_admin_sdk
import os
import configparser
import yaml
import sys
from garage_admin_sdk.apis import *
from garage_admin_sdk.models import *
from kubernetes import client
from kubernetes import config as k8sconfig


def str2bool(v):
  return v.lower() in ("yes", "true", "t", "1")

def convert_to_bytes(capacity):
    try:
        float(capacity)
        return int(float(capacity))
    except ValueError:
      multipliers = {
          '': 1,
          'Mi': 1024 ** 2,
          'Gi': 1024 ** 3,
      }

      num = float(capacity[:-2])
      unit = capacity[-2:]

      if unit not in multipliers:
          raise ValueError("Invalid unit")

      return int(num * multipliers[unit])


configuration = garage_admin_sdk.Configuration(
  host = "http://localhost:3903/v1",
  access_token = os.getenv("GARAGE_ADMIN_TOKEN")
)

config = configparser.RawConfigParser()
config.read('codefresh-garage.properties')

api = garage_admin_sdk.ApiClient(configuration)
nodes, layout, keys, buckets = NodesApi(api), LayoutApi(api), KeyApi(api), BucketApi(api)

statusLayout = layout.get_layout()
allNodes = nodes.get_nodes()

for node in allNodes["known_nodes"]:
    layout.add_layout([
      NodeRoleChange(
        id = node.id,
        zone = "codefresh-gitops-runtime",
        capacity = convert_to_bytes(config.get("garage","node_capacity")),
        tags = [ "codefresh-gitops-runtime" ],
      )
    ])

layout.apply_layout(LayoutVersion(
  version = statusLayout.version + 1
))

print("INFO: Garage cluster layout applied")

kinfo = None  

for key in keys.list_keys():
    if key.name == "argo-workflows":
        kinfo = keys.get_key(id=key.id, show_secret_key="true")

if kinfo == None:
    kinfo = keys.add_key(AddKeyRequest(name="argo-workflows"))

bucketInfo = None
#binfo = buckets.create_bucket(CreateBucketRequest(global_alias="argo-workflows"))

for bucket in buckets.list_buckets():
    if "argo-workflows" in bucket.global_aliases:
        bucketInfo = buckets.get_bucket_info(id=bucket.id)
        print("INFO: Argo workflows bucket exists, proceeding")

if bucketInfo == None:
    bucketInfo = buckets.create_bucket(CreateBucketRequest(global_alias="argo-workflows"))

# Give permissions
buckets.allow_bucket_key(AllowBucketKeyRequest(
  bucket_id=bucketInfo.id,
  access_key_id=kinfo.access_key_id,
  permissions=AllowBucketKeyRequestPermissions(read=True, write=True, owner=True),
))

print("INFO: Permissions to bucket granted")

####
# Argo workflows configurations - KubeAPI secret and CM
#####
bucketCredsConfig = {"accessKey": kinfo.access_key_id, "secretKey": kinfo.secret_access_key}

credsSecret = client.V1Secret(api_version="v1", 
                              kind="Secret", 
                              string_data=bucketCredsConfig, 
                              metadata=client.V1ObjectMeta(name="argo-workflows-garage-creds"))

argoWorkflowsS3Config = { "s3": {
                                  "endpoint": config.get("garage","s3_api_url"),
                                  "insecure": str2bool(config.get("garage","insecure")),
                                  "bucket": "argo-workflows",
                                  "archiveLogs": True,
                                  "accessKeySecret": {"name": "argo-workflows-garage-creds", "key": "accessKey"},
                                  "secretKeySecret": {"name": "argo-workflows-garage-creds", "key": "secretKey"}}
                        }

strWorkflowsCM =  yaml.dump(argoWorkflowsS3Config, default_flow_style=False)

# Create a ConfigMap object
cm = client.V1ConfigMap(
      api_version="v1",
      kind="ConfigMap",
      data={"garage": strWorkflowsCM},
      metadata=client.V1ObjectMeta(name="artifact-repositories",annotations={"workflows.argoproj.io/default-artifact-repository": "garage"})
  )

# Load kubeconfig file
k8sconfig.load_kube_config()

# Create a Kubernetes client
core_v1 = client.CoreV1Api()

try:
    # Apply the ConfigMap
    response = core_v1.patch_namespaced_secret(name="argo-workflows-garage-creds",namespace=config.get("workflows-cm","namespace"), body=credsSecret)
    print("Secret applied successfully.")
except client.exceptions.ApiException as e:
    if e.status == 404:
        response = core_v1.create_namespaced_secret(namespace=config.get("workflows-cm","namespace"), body=credsSecret)(
            namespace=config.get("workflows-cm","namespace"), body=cm
        )
        print("Secret created successfully.")
    else:
      print(f"Exception when calling CoreV1Api->create_namespaced_secret/patch_namespaced_secret: {e}")

try:
    # Apply the ConfigMap
    response = core_v1.patch_namespaced_config_map(name=config.get("workflows-cm","name"),namespace=config.get("workflows-cm","namespace"), body=cm)
    print("ConfigMap applied successfully.")
except client.exceptions.ApiException as e:
    if e.status == 404:
        response = core_v1.create_namespaced_config_map(
            namespace=config.get("workflows-cm","namespace"), body=cm
        )
        print("ConfigMap created successfully.")
    else:
      print(f"Exception when calling CoreV1Api->create_namespaced_config_map/patch_namespaced_config_map: {e}")




