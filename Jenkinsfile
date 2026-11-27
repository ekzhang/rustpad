#!groovy
// Builds the Rustpad and runner images into ECR, tagged with the commit SHA,
// then releases both to dev through ArgoCD. Runs on main only; other branches
// pass without doing anything.
//
// Uses the shared jobs on build.viacom.com:
//   utilities/ecr-build-and-publish   builds and pushes <image>:<sha>
//   utilities/eks-rolling-update-deploy  sets the tag in devops-k8s-applications
//                                        and waits for ArgoCD

import groovy.transform.Field

@Field String githubRepository = 'viacomcbs/rustpad'
@Field String architecture = 'all'
@Field String valuesRepository = 'viacomcbs/devops-k8s-applications'
@Field String contactEmail = 'devops@viacom.com'

@Field Map apps = [
    'rustpad-runner': [
        image: 'devops/devops-rustpad-runner',
        dockerfileDir: 'runner',
        dockerfileName: 'Dockerfile',
        buildContext: 'runner',
        shaBuildArg: '',
    ],
    'rustpad': [
        image: 'devops/devops-rustpad',
        dockerfileDir: '.',
        dockerfileName: 'Dockerfile.ci',
        buildContext: '',
        shaBuildArg: 'VITE_SHA', // shown in the footer
    ],
]

pipeline {
    agent none
    parameters {
        booleanParam name: 'BUILD_IMAGES',
            defaultValue: true,
            description: 'Untick to release images already in ECR for this commit. Tags are immutable, so a second build of the same commit fails.'
    }
    options {
        ansiColor 'xterm'
        buildDiscarder logRotator(numToKeepStr: '30')
        disableConcurrentBuilds()
        timeout(time: 90, unit: 'MINUTES')
    }
    stages {
        stage('Resolve commit') {
            when {
                beforeAgent true
                branch 'main'
            }
            agent { label 'generic_v3 && 4GB' }
            steps {
                setImageTag()
            }
        }
        stage('Dev') {
            when { branch 'main' }
            environment {
                // Builds go to nonprod; utilities/ecr-copy promotes a released tag to prod.
                AWS_ACCOUNT_NAME = 'viacbs-networkstreaming-nonprod'
                REGISTRY = '877557201532.dkr.ecr.us-east-1.amazonaws.com'
                ENVIRONMENT_SHORT = 'dev'
            }
            stages {
                stage('Build images') {
                    when { expression { params.BUILD_IMAGES } }
                    parallel {
                        stage('Build rustpad-runner') {
                            steps {
                                buildImage 'rustpad-runner'
                            }
                        }
                        stage('Build rustpad') {
                            steps {
                                buildImage 'rustpad'
                            }
                        }
                    }
                }
                // Rustpad calls the runner, so the runner is released first.
                stage('Release rustpad-runner') {
                    steps {
                        releaseImage 'rustpad-runner'
                    }
                }
                stage('Release rustpad') {
                    steps {
                        releaseImage 'rustpad'
                    }
                }
            }
        }
    }
}

// GIT_COMMIT exists only after checkout on an agent, and a stage-level
// environment block would not outlive the stage, so the tag goes into env here.
void setImageTag() {
    env.IMAGE_TAG = env.GIT_COMMIT
    currentBuild.description = env.IMAGE_TAG.take(7)
}

void buildImage(String name) {
    Map app = apps[name]
    String buildArgs = app.shaBuildArg ? "${app.shaBuildArg}=${env.IMAGE_TAG}" : ''
    build job: 'utilities/ecr-build-and-publish', parameters: [
        string(name: 'github_repository', value: githubRepository),
        string(name: 'branch', value: env.IMAGE_TAG),
        string(name: 'architecture', value: architecture),
        string(name: 'image', value: app.image),
        string(name: 'tag', value: env.IMAGE_TAG),
        string(name: 'dockerfile_dir', value: app.dockerfileDir),
        string(name: 'dockerfile_name', value: app.dockerfileName),
        string(name: 'build_context', value: app.buildContext),
        string(name: 'aws_account_name', value: env.AWS_ACCOUNT_NAME),
        string(name: 'aws_region', value: 'us-east-1'),
        booleanParam(name: 'create_repo', value: false),
        string(name: 'build_args', value: buildArgs),
        string(name: 'contact_email', value: contactEmail),
    ]
}

void releaseImage(String name) {
    Map app = apps[name]
    build job: 'utilities/eks-rolling-update-deploy', parameters: [
        string(name: 'github_repository', value: valuesRepository),
        string(name: 'branch', value: 'main'),
        string(name: 'helm_values_file', value: "${name}/use1-${env.ENVIRONMENT_SHORT}.yaml"),
        string(name: 'key_to_update', value: "pod.containers.${name}.image"),
        string(name: 'container_image_name', value: "${env.REGISTRY}/${app.image}"),
        string(name: 'container_image_tag', value: env.IMAGE_TAG),
        string(name: 'patch_values', value: '{}'),
        string(name: 'argocd_application_name', value: "use1-devops-${name}-${env.ENVIRONMENT_SHORT}"),
        booleanParam(name: 'restart_deployment', value: false),
        string(name: 'timeout', value: '600'),
        string(name: 'contact_email', value: contactEmail),
    ]
}
