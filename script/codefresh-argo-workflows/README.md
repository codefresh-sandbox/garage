Those are the results of a one day POC.

The python script needs to be executed after runtime is installed

It does 3 things:
1. Configures garage cluster
2. Creates a bucket and an S3 API key
3. Creates secret and workflows artifacts CM

Prior to running the script - if executing locally
1. To get the token to authenticate to garage admin api: `export GARAGE_ADMIN_TOKEN=$(kubectl -n codefresh-gitops-runtime get secret garage-codefresh-admin -o=jsonpath='{.data.token}' | base64 --decode)`
2. Port forward 3093 port from garage