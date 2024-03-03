## Those are the results of a one day POC.

The python script needs to be executed after runtime is installed

It does 3 things:
1. Configures garage cluster
2. Creates a bucket and an S3 API key
3. Creates secret and workflows artifacts CM

Prior to running the script - if executing locally
1. To get the token to authenticate to garage admin api: `export GARAGE_ADMIN_TOKEN=$(kubectl -n codefresh-gitops-runtime get secret garage-codefresh-admin -o=jsonpath='{.data.token}' | base64 --decode)`
2. Port forward 3093 port from garage

## How ro run POC scenario:
1. Deploy gitops runtime
2. Deploy garage to the runtime namespace by running helm install from script/helm
3. Execute prerequisites for running script (above)
4. Run the script