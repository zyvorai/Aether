# Example Dockerfile for testing Aether
FROM nginx:alpine

# Add a simple health check endpoint
RUN echo '<html><body><h1>Hello from Aether!</h1></body></html>' > /usr/share/nginx/html/index.html
RUN echo 'OK' > /usr/share/nginx/html/health
RUN echo 'READY' > /usr/share/nginx/html/ready

# Expose port 80
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
