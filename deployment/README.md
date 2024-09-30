# Installation

A basic installation can be invoked with `kubectl apply -f ./kubernetes.yaml`

```yaml
$ kubectl apply -f ./kubernetes.yaml
deployment.apps/rustpad-deployment created
service/rustpad-service created
```

We can the verify it is running
```
$ kubectl get svc rustpad-service
NAME              TYPE        CLUSTER-IP     EXTERNAL-IP   PORT(S)   AGE
rustpad-service   ClusterIP   10.43.83.244   <none>        80/TCP    17s
$ kubectl get pods -l app=rustpad
NAME                                 READY   STATUS    RESTARTS   AGE
rustpad-deployment-8799874d6-g2jxb   1/1     Running   0          26s
```

Then use port-forward to access
```
$ kubectl port-forward svc/rustpad-service 8888:80
Forwarding from 127.0.0.1:8888 -> 3030
Forwarding from [::1]:8888 -> 3030
Handling connection for 8888
```

## Ingress

Ingress depends much on your cluster-issuer and DNS names

In the example `ingress.yaml` my ClusterIssuer is 'myclusterissuer', my ingress provider is 'nginx' and my DNS name is 'rustpad.example.com'

Replace with your own to create a proper TLS ingress for Rustpad, then apply

```
$ kubectl apply -f ./ingress.yaml
ingress.networking.k8s.io/rustpadgcpingress created
```

Note: if you use a different Ingress provider than Nginx, ensure it can support websocket forwarding if you want it to "connect" to the backend server.