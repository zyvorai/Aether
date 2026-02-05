# Example Dockerfile for testing Orchestr8
FROM nginx:alpine

# Add a simple health check endpoint
RUN echo '<html><body><h1>Hello from Orchestr8!</h1></body></html>' > /usr/share/nginx/html/index.html
RUN echo 'OK' > /usr/share/nginx/html/health
RUN echo 'READY' > /usr/share/nginx/html/ready

# Expose port 80
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
